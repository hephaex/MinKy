"""Timeline and date-based document endpoints."""
from flask import Blueprint, request, jsonify
from flask_jwt_extended import jwt_required
from sqlalchemy import or_, extract, and_, func
from app import db, limiter
from app.models.document import Document
from app.utils.auth import get_current_user_id
from app.utils.responses import paginate_query
import logging

logger = logging.getLogger(__name__)

documents_timeline_bp = Blueprint('documents_timeline', __name__)


@documents_timeline_bp.route('/documents/by-date', methods=['GET'])
@limiter.limit("60 per minute")
@jwt_required(optional=True)
def get_documents_by_date():
    """Get documents by date filter"""
    try:
        current_user_id = get_current_user_id()
        date_key = request.args.get('date_key')
        page = request.args.get('page', 1, type=int)
        per_page = request.args.get('per_page', 50, type=int)

        # SECURITY: Enforce pagination bounds to prevent resource exhaustion
        page = max(1, page)
        per_page = max(1, min(per_page, 100))

        if not date_key:
            return jsonify({'error': 'date_key parameter is required'}), 400

        # SECURITY: Validate date_key length to prevent DoS
        if len(date_key) > 10:  # YYYY-MM-DD is 10 chars max
            return jsonify({'error': 'Invalid date_key format'}), 400

        # Parse date_key (supports YYYY, YYYY-MM, YYYY-MM-DD formats)
        date_parts = date_key.split('-')
        if len(date_parts) < 1:
            return jsonify({'error': 'Invalid date_key format'}), 400

        try:
            year = int(date_parts[0])
            month = int(date_parts[1]) if len(date_parts) > 1 else None
            day = int(date_parts[2]) if len(date_parts) > 2 else None

            # SECURITY: Validate date ranges to prevent invalid queries
            if not (1900 <= year <= 2100):
                return jsonify({'error': 'Year must be between 1900 and 2100'}), 400
            if month is not None and not (1 <= month <= 12):
                return jsonify({'error': 'Month must be between 1 and 12'}), 400
            if day is not None and not (1 <= day <= 31):
                return jsonify({'error': 'Day must be between 1 and 31'}), 400
        except ValueError:
            return jsonify({'error': 'Invalid date_key format'}), 400

        # Build query conditions
        if current_user_id:
            base_query = Document.query.filter(
                or_(Document.user_id == current_user_id, Document.is_public == True)
            )
        else:
            base_query = Document.query.filter_by(is_public=True)

        # Date filtering
        filters = [extract('year', Document.created_at) == year]

        if month is not None:
            filters.append(extract('month', Document.created_at) == month)

        if day is not None:
            filters.append(extract('day', Document.created_at) == day)

        query = base_query.filter(and_(*filters)).order_by(Document.created_at.desc())

        return paginate_query(
            query, page, per_page,
            serializer_func=lambda d: d.to_dict(),
            items_key='documents',
            extra_fields={'date_key': date_key}
        )

    except Exception as e:
        logger.error("Error getting documents by date: %s", e)
        return jsonify({'error': 'Internal server error'}), 500


def _group_by_year(timeline: dict, created_at) -> None:
    """Group document by year"""
    year_key = str(created_at.year)
    year_label = f"{created_at.year}년"

    if year_key not in timeline:
        timeline[year_key] = {
            'key': year_key,
            'label': year_label,
            'count': 0
        }

    timeline[year_key]['count'] += 1


def _group_by_month(timeline: dict, created_at) -> None:
    """Group document by month"""
    year_key = str(created_at.year)
    year_label = f"{created_at.year}년"
    month_key = f"{created_at.year}-{created_at.month:02d}"
    month_label = f"{created_at.month}월"

    if year_key not in timeline:
        timeline[year_key] = {
            'key': year_key,
            'label': year_label,
            'count': 0,
            'children': {}
        }

    if month_key not in timeline[year_key]['children']:
        timeline[year_key]['children'][month_key] = {
            'key': month_key,
            'label': month_label,
            'count': 0
        }

    timeline[year_key]['count'] += 1
    timeline[year_key]['children'][month_key]['count'] += 1


def _group_by_day(timeline: dict, created_at) -> None:
    """Group document by day"""
    year_key = str(created_at.year)
    year_label = f"{created_at.year}년"
    month_key = f"{created_at.year}-{created_at.month:02d}"
    month_label = f"{created_at.month}월"
    day_key = f"{created_at.year}-{created_at.month:02d}-{created_at.day:02d}"
    day_label = f"{created_at.day}일"

    if year_key not in timeline:
        timeline[year_key] = {
            'key': year_key,
            'label': year_label,
            'count': 0,
            'children': {}
        }

    if month_key not in timeline[year_key]['children']:
        timeline[year_key]['children'][month_key] = {
            'key': month_key,
            'label': month_label,
            'count': 0,
            'children': {}
        }

    if day_key not in timeline[year_key]['children'][month_key]['children']:
        timeline[year_key]['children'][month_key]['children'][day_key] = {
            'key': day_key,
            'label': day_label,
            'count': 0
        }

    timeline[year_key]['count'] += 1
    timeline[year_key]['children'][month_key]['count'] += 1
    timeline[year_key]['children'][month_key]['children'][day_key]['count'] += 1


def _build_timeline(documents: list[Document], group_by: str) -> dict:
    """Build timeline data from documents"""
    timeline = {}
    grouping_functions = {
        'year': _group_by_year,
        'month': _group_by_month,
        'day': _group_by_day
    }

    group_func = grouping_functions.get(group_by)
    if not group_func:
        return timeline

    for doc in documents:
        if not doc.created_at:
            continue
        group_func(timeline, doc.created_at)

    return timeline


@documents_timeline_bp.route('/documents/timeline', methods=['GET'])
@limiter.limit("20 per minute")
@jwt_required(optional=True)
def get_documents_timeline():
    """Get documents timeline data"""
    try:
        current_user_id = get_current_user_id()
        group_by = request.args.get('group_by', 'month')

        # SECURITY: Validate group_by parameter against whitelist
        VALID_GROUP_BY = {'year', 'month', 'day'}
        if group_by not in VALID_GROUP_BY:
            return jsonify({
                'error': f'Invalid group_by value. Must be one of: {", ".join(VALID_GROUP_BY)}'
            }), 400

        max_documents = 5000
        if current_user_id:
            documents = Document.query.filter(
                or_(Document.user_id == current_user_id, Document.is_public == True)
            ).order_by(Document.created_at.desc()).limit(max_documents).all()
        else:
            documents = Document.query.filter_by(is_public=True).order_by(Document.created_at.desc()).limit(max_documents).all()

        timeline = _build_timeline(documents, group_by)

        return jsonify({
            'timeline': timeline,
            'group_by': group_by,
            'total_documents': len(documents)
        })

    except Exception as e:
        logger.error("Error getting documents timeline: %s", e)
        return jsonify({'error': 'Internal server error'}), 500


def _get_visible_documents(current_user_id):
    """Get base filter for visible documents"""
    if current_user_id:
        return or_(
            Document.user_id == current_user_id,
            Document.is_public == True
        )
    return Document.is_public == True


def _build_tag_tree(base_filter):
    """Build tree structure grouped by tags (optimized to avoid N+1 queries)"""
    from app.models.tag import Tag, document_tags
    from collections import defaultdict

    # Single query to get all documents with their tags
    docs_with_tags = (
        db.session.query(
            Document.id,
            Document.title,
            Document.updated_at,
            Tag.id.label('tag_id'),
            Tag.name.label('tag_name'),
            Tag.slug.label('tag_slug'),
            Tag.color.label('tag_color')
        )
        .join(document_tags, Document.id == document_tags.c.document_id)
        .join(Tag, Tag.id == document_tags.c.tag_id)
        .filter(base_filter)
        .order_by(Document.updated_at.desc())
        .all()
    )

    # Build tag -> documents mapping in memory
    tag_docs = defaultdict(list)
    tag_info = {}

    for doc_id, doc_title, doc_updated, tag_id, tag_name, tag_slug, tag_color in docs_with_tags:
        if tag_id not in tag_info:
            tag_info[tag_id] = {
                'name': tag_name,
                'slug': tag_slug,
                'color': tag_color
            }
        # Avoid duplicates (document may appear multiple times if it has multiple tags)
        doc_entry = {'id': doc_id, 'title': doc_title, 'updated_at': doc_updated}
        if doc_entry not in tag_docs[tag_id]:
            tag_docs[tag_id].append(doc_entry)

    # Build tree sorted by document count
    tree = []
    sorted_tags = sorted(tag_info.items(), key=lambda x: len(tag_docs[x[0]]), reverse=True)

    for tag_id, info in sorted_tags:
        children = [
            {
                'id': f'doc-{doc["id"]}',
                'label': doc['title'],
                'type': 'document',
                'children': [],
                'count': 0,
                'documentId': doc['id']
            }
            for doc in tag_docs[tag_id]
        ]

        tree.append({
            'id': f'tag-{info["slug"]}',
            'label': info['name'],
            'type': 'tag',
            'children': children,
            'count': len(children),
            'documentId': None,
            'color': info['color']
        })

    untagged_docs = (
        Document.query
        .filter(base_filter, ~Document.tags.any())
        .order_by(Document.updated_at.desc())
        .all()
    )

    if untagged_docs:
        untagged_children = [
            {
                'id': f'doc-{doc.id}',
                'label': doc.title,
                'type': 'document',
                'children': [],
                'count': 0,
                'documentId': doc.id
            }
            for doc in untagged_docs
        ]
        tree.append({
            'id': 'tag-untagged',
            'label': '태그 없음',
            'type': 'tag',
            'children': untagged_children,
            'count': len(untagged_docs),
            'documentId': None,
            'color': '#888888'
        })

    return tree


def _build_date_tree(base_filter, max_documents=5000):
    """Build tree structure grouped by date"""
    # SECURITY: Limit query results to prevent resource exhaustion
    documents = (
        Document.query
        .filter(base_filter)
        .order_by(Document.created_at.desc())
        .limit(max_documents)
        .all()
    )

    years = {}
    for doc in documents:
        created_at = doc.created_at
        if not created_at:
            continue

        year_key = str(created_at.year)
        month_key = f"{created_at.year}-{created_at.month:02d}"
        month_label = f"{created_at.month}월"

        if year_key not in years:
            years[year_key] = {
                'label': f"{created_at.year}년",
                'months': {}
            }

        if month_key not in years[year_key]['months']:
            years[year_key]['months'][month_key] = {
                'label': month_label,
                'docs': []
            }

        years[year_key]['months'][month_key]['docs'].append(doc)

    tree = []
    for year_key in sorted(years.keys(), reverse=True):
        year_data = years[year_key]
        month_nodes = []

        for month_key in sorted(year_data['months'].keys(), reverse=True):
            month_data = year_data['months'][month_key]
            doc_nodes = [
                {
                    'id': f'doc-{doc.id}',
                    'label': doc.title,
                    'type': 'document',
                    'children': [],
                    'count': 0,
                    'documentId': doc.id
                }
                for doc in month_data['docs']
            ]

            month_nodes.append({
                'id': f'date-{month_key}',
                'label': month_data['label'],
                'type': 'month',
                'children': doc_nodes,
                'count': len(doc_nodes),
                'documentId': None
            })

        year_doc_count = sum(m['count'] for m in month_nodes)
        tree.append({
            'id': f'date-{year_key}',
            'label': year_data['label'],
            'type': 'year',
            'children': month_nodes,
            'count': year_doc_count,
            'documentId': None
        })

    return tree


@documents_timeline_bp.route('/documents/tree', methods=['GET'])
@limiter.limit("30 per minute")
@jwt_required(optional=True)
def get_documents_tree():
    """Get documents tree structure (by tag or by date)"""
    try:
        current_user_id = get_current_user_id()
        mode = request.args.get('mode', 'by-tag')
        base_filter = _get_visible_documents(current_user_id)

        # SECURITY: Validate mode parameter against whitelist (don't echo user input)
        VALID_MODES = {'by-tag', 'by-date'}
        if mode not in VALID_MODES:
            return jsonify({
                'error': f'Invalid mode. Valid options are: {", ".join(VALID_MODES)}'
            }), 400

        if mode == 'by-tag':
            tree = _build_tag_tree(base_filter)
        else:  # mode == 'by-date'
            tree = _build_date_tree(base_filter)

        total = sum(node['count'] for node in tree)

        return jsonify({
            'tree': tree,
            'mode': mode,
            'total_documents': total
        })

    except Exception as e:
        logger.error("Error building document tree: %s", e)
        return jsonify({'error': 'Internal server error'}), 500
