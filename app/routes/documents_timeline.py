"""Timeline and date-based document endpoints."""
from flask import Blueprint, request, jsonify
from flask_jwt_extended import jwt_required
from sqlalchemy import or_, extract, and_, func
from app import db
from app.models.document import Document
from app.utils.auth import get_current_user_id
from app.utils.responses import paginate_query
import logging

logger = logging.getLogger(__name__)

documents_timeline_bp = Blueprint('documents_timeline', __name__)


@documents_timeline_bp.route('/documents/by-date', methods=['GET'])
@jwt_required(optional=True)
def get_documents_by_date():
    """Get documents by date filter"""
    try:
        current_user_id = get_current_user_id()
        date_key = request.args.get('date_key')
        page = request.args.get('page', 1, type=int)
        per_page = request.args.get('per_page', 50, type=int)

        if not date_key:
            return jsonify({'error': 'date_key parameter is required'}), 400

        # Parse date_key (supports YYYY, YYYY-MM, YYYY-MM-DD formats)
        date_parts = date_key.split('-')
        if len(date_parts) < 1:
            return jsonify({'error': 'Invalid date_key format'}), 400

        try:
            year = int(date_parts[0])
            month = int(date_parts[1]) if len(date_parts) > 1 else None
            day = int(date_parts[2]) if len(date_parts) > 2 else None
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


@documents_timeline_bp.route('/documents/timeline', methods=['GET'])
@jwt_required(optional=True)
def get_documents_timeline():
    """Get documents timeline data"""
    try:
        current_user_id = get_current_user_id()
        group_by = request.args.get('group_by', 'month')  # 'month', 'year', 'day'

        # Get visible documents (max 5000)
        max_documents = 5000
        if current_user_id:
            documents = Document.query.filter(
                or_(Document.user_id == current_user_id, Document.is_public == True)
            ).order_by(Document.created_at.desc()).limit(max_documents).all()
        else:
            documents = Document.query.filter_by(is_public=True).order_by(Document.created_at.desc()).limit(max_documents).all()

        # Build timeline data
        timeline = {}

        for doc in documents:
            created_at = doc.created_at
            if not created_at:
                continue

            if group_by == 'month':
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

            elif group_by == 'year':
                year_key = str(created_at.year)
                year_label = f"{created_at.year}년"

                if year_key not in timeline:
                    timeline[year_key] = {
                        'key': year_key,
                        'label': year_label,
                        'count': 0
                    }

                timeline[year_key]['count'] += 1

            elif group_by == 'day':
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

        return jsonify({
            'timeline': timeline,
            'group_by': group_by,
            'total_documents': len(documents)
        })

    except Exception as e:
        logger.error("Error getting documents timeline: %s", e)
        return jsonify({'error': 'Internal server error'}), 500


@documents_timeline_bp.route('/documents/tree', methods=['GET'])
@jwt_required(optional=True)
def get_documents_tree():
    """Get documents tree structure (by tag or by date)"""
    try:
        from app.models.tag import Tag, document_tags

        current_user_id = get_current_user_id()
        mode = request.args.get('mode', 'by-tag')

        if current_user_id:
            base_filter = or_(
                Document.user_id == current_user_id,
                Document.is_public == True
            )
        else:
            base_filter = Document.is_public == True

        if mode == 'by-tag':
            # Group by tag: tag -> documents
            tag_doc_counts = (
                db.session.query(
                    Tag.id,
                    Tag.name,
                    Tag.slug,
                    Tag.color,
                    func.count(Document.id).label('doc_count')
                )
                .join(document_tags, Tag.id == document_tags.c.tag_id)
                .join(Document, Document.id == document_tags.c.document_id)
                .filter(base_filter)
                .group_by(Tag.id, Tag.name, Tag.slug, Tag.color)
                .order_by(func.count(Document.id).desc())
                .all()
            )

            tree = []
            for tag_id, tag_name, tag_slug, tag_color, doc_count in tag_doc_counts:
                docs_query = (
                    Document.query
                    .join(document_tags, Document.id == document_tags.c.document_id)
                    .filter(
                        document_tags.c.tag_id == tag_id,
                        base_filter
                    )
                    .order_by(Document.updated_at.desc())
                    .all()
                )

                children = [
                    {
                        'id': f'doc-{doc.id}',
                        'label': doc.title,
                        'type': 'document',
                        'children': [],
                        'count': 0,
                        'documentId': doc.id
                    }
                    for doc in docs_query
                ]

                tree.append({
                    'id': f'tag-{tag_slug}',
                    'label': tag_name,
                    'type': 'tag',
                    'children': children,
                    'count': doc_count,
                    'documentId': None,
                    'color': tag_color
                })

            # Untagged documents
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

            total = sum(node['count'] for node in tree)

        elif mode == 'by-date':
            # Group by date: year -> month -> documents
            documents = (
                Document.query
                .filter(base_filter)
                .order_by(Document.created_at.desc())
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

            total = sum(node['count'] for node in tree)

        else:
            return jsonify({'error': f'Invalid mode: {mode}. Use by-tag or by-date'}), 400

        return jsonify({
            'tree': tree,
            'mode': mode,
            'total_documents': total
        })

    except Exception as e:
        logger.error("Error building document tree: %s", e)
        return jsonify({'error': 'Internal server error'}), 500
