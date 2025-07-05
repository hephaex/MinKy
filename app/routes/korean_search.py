from flask import Blueprint, request, jsonify, current_app
from flask_jwt_extended import jwt_required, get_jwt_identity, jwt_required_optional
from app.models.document import Document
from app.models.user import User
from app.utils.korean_text import korean_processor, search_korean_text, process_korean_document
from app.services.opensearch_service import get_opensearch_service
from app.middleware.security import rate_limit_api, rate_limit_search, validate_request_security, audit_log
from marshmallow import Schema, fields, ValidationError
from sqlalchemy import or_, and_
from datetime import datetime
import json

korean_search_bp = Blueprint('korean_search', __name__)

class KoreanSearchSchema(Schema):
    query = fields.Str(required=True, validate=lambda x: len(x.strip()) > 0)
    language = fields.Str(missing='auto', validate=lambda x: x in ['auto', 'korean', 'english', 'mixed'])
    tags = fields.List(fields.Str(), missing=[])
    author = fields.Str(allow_none=True)
    date_from = fields.DateTime(allow_none=True)
    date_to = fields.DateTime(allow_none=True)
    page = fields.Int(missing=1, validate=lambda x: x > 0)
    per_page = fields.Int(missing=20, validate=lambda x: 1 <= x <= 100)
    use_opensearch = fields.Bool(missing=False)

@korean_search_bp.route('/search/korean', methods=['POST'])
@jwt_required_optional
@rate_limit_search("30 per minute")
@validate_request_security
@audit_log("korean_text_search")
def korean_text_search():
    """한국어 텍스트 전용 검색"""
    current_user_id = get_jwt_identity()
    
    data = request.get_json()
    if not data:
        return jsonify({'error': 'Request body required'}), 400
    
    # 데이터 검증
    schema = KoreanSearchSchema()
    try:
        validated_data = schema.load(data)
    except ValidationError as e:
        return jsonify({'error': 'Invalid data', 'details': e.messages}), 400
    
    query = validated_data['query']
    page = validated_data['page']
    per_page = validated_data['per_page']
    use_opensearch = validated_data['use_opensearch']
    
    try:
        # OpenSearch 사용 여부 결정
        opensearch_service = get_opensearch_service()
        if use_opensearch and opensearch_service:
            return _search_with_opensearch(validated_data, current_user_id)
        else:
            return _search_with_postgresql(validated_data, current_user_id)
            
    except Exception as e:
        current_app.logger.error(f"Korean search failed: {str(e)}")
        return jsonify({'error': 'Search failed', 'details': str(e)}), 500

def _search_with_opensearch(search_params: dict, user_id: int = None) -> dict:
    """OpenSearch를 사용한 검색"""
    opensearch_service = get_opensearch_service()
    
    filters = {}
    if search_params.get('tags'):
        filters['tags'] = search_params['tags']
    if search_params.get('author'):
        filters['author'] = search_params['author']
    if search_params.get('date_from'):
        filters['date_from'] = search_params['date_from'].isoformat()
    if search_params.get('date_to'):
        filters['date_to'] = search_params['date_to'].isoformat()
    if search_params.get('language') != 'auto':
        filters['language'] = search_params['language']
    
    results = opensearch_service.search_documents(
        query=search_params['query'],
        filters=filters,
        page=search_params['page'],
        per_page=search_params['per_page'],
        user_id=user_id
    )
    
    return jsonify({
        'documents': results['documents'],
        'pagination': {
            'page': results['page'],
            'per_page': results['per_page'],
            'total': results['total'],
            'pages': results['pages']
        },
        'search_engine': 'opensearch',
        'query_processed': True
    })

def _search_with_postgresql(search_params: dict, user_id: int = None) -> dict:
    """PostgreSQL을 사용한 한국어 검색"""
    query = search_params['query']
    page = search_params['page']
    per_page = search_params['per_page']
    
    # 언어 감지
    detected_language = korean_processor.detect_language(query)
    
    # 기본 쿼리 구성
    base_query = Document.query
    
    # 접근 권한 필터
    if user_id:
        base_query = base_query.filter(
            or_(Document.is_public == True, Document.user_id == user_id)
        )
    else:
        base_query = base_query.filter(Document.is_public == True)
    
    # 추가 필터 적용
    if search_params.get('tags'):
        from app.models.tag import Tag
        for tag_name in search_params['tags']:
            tag = Tag.query.filter_by(name=tag_name).first()
            if tag:
                base_query = base_query.filter(Document.tags.contains(tag))
    
    if search_params.get('author'):
        base_query = base_query.filter(Document.author.ilike(f"%{search_params['author']}%"))
    
    if search_params.get('date_from'):
        base_query = base_query.filter(Document.updated_at >= search_params['date_from'])
    
    if search_params.get('date_to'):
        base_query = base_query.filter(Document.updated_at <= search_params['date_to'])
    
    # 한국어 검색 수행
    if detected_language == 'korean':
        # 한국어 토큰 기반 검색
        query_tokens = korean_processor.tokenize(query)
        if query_tokens:
            search_conditions = []
            for token in query_tokens:
                search_conditions.extend([
                    Document.title.ilike(f"%{token}%"),
                    Document.markdown_content.ilike(f"%{token}%")
                ])
            
            if search_conditions:
                base_query = base_query.filter(or_(*search_conditions))
    else:
        # 기본 PostgreSQL 전문 검색
        from sqlalchemy import func
        base_query = base_query.filter(
            func.to_tsvector('english', Document.title + ' ' + Document.markdown_content).match(
                func.plainto_tsquery('english', query)
            )
        )
    
    # 페이지네이션 적용
    paginated_results = base_query.order_by(Document.updated_at.desc()).paginate(
        page=page, per_page=per_page, error_out=False
    )
    
    # 결과 처리
    documents = []
    for doc in paginated_results.items:
        doc_dict = doc.to_dict()
        
        # 검색어 하이라이트
        if detected_language == 'korean':
            doc_dict['highlighted_content'] = korean_processor.highlight_korean_text(
                doc.markdown_content, query
            )
        else:
            # 기본 하이라이트
            content = doc.markdown_content[:200] + '...' if len(doc.markdown_content) > 200 else doc.markdown_content
            doc_dict['highlighted_content'] = content
        
        documents.append(doc_dict)
    
    return jsonify({
        'documents': documents,
        'pagination': {
            'page': page,
            'per_page': per_page,
            'total': paginated_results.total,
            'pages': paginated_results.pages
        },
        'search_engine': 'postgresql',
        'detected_language': detected_language,
        'query_tokens': korean_processor.tokenize(query) if detected_language == 'korean' else []
    })

@korean_search_bp.route('/search/suggest-tags', methods=['GET'])
@jwt_required_optional
@rate_limit_api("60 per minute")
@validate_request_security
def suggest_korean_tags():
    """한국어 태그 자동완성"""
    query = request.args.get('q', '').strip()
    limit = min(int(request.args.get('limit', 10)), 20)
    
    if not query:
        return jsonify({'suggestions': []})
    
    try:
        # OpenSearch 사용 가능한 경우
        opensearch_service = get_opensearch_service()
        if opensearch_service:
            suggestions = opensearch_service.suggest_tags(query, limit)
            return jsonify({
                'suggestions': suggestions,
                'source': 'opensearch'
            })
        
        # PostgreSQL 폴백
        from app.models.tag import Tag
        
        # 한국어 처리
        if korean_processor.detect_language(query) == 'korean':
            # 한국어 태그 검색 (부분 매칭)
            tags = Tag.query.filter(
                Tag.name.ilike(f"%{query}%")
            ).order_by(Tag.usage_count.desc()).limit(limit).all()
        else:
            # 영어/숫자 태그 검색
            tags = Tag.query.filter(
                Tag.name.ilike(f"{query}%")
            ).order_by(Tag.usage_count.desc()).limit(limit).all()
        
        suggestions = [tag.name for tag in tags]
        
        return jsonify({
            'suggestions': suggestions,
            'source': 'postgresql'
        })
        
    except Exception as e:
        current_app.logger.error(f"Tag suggestion failed: {str(e)}")
        return jsonify({'suggestions': [], 'error': str(e)})

@korean_search_bp.route('/documents/<int:document_id>/analyze-korean', methods=['POST'])
@jwt_required
@rate_limit_api("20 per minute")
@validate_request_security
@audit_log("analyze_korean_document")
def analyze_korean_document(document_id):
    """문서의 한국어 분석 결과 반환"""
    current_user_id = get_jwt_identity()
    user = User.query.get(current_user_id)
    
    if not user:
        return jsonify({'error': 'User not found'}), 404
    
    document = Document.query.get_or_404(document_id)
    
    # 접근 권한 확인
    if not document.can_view(current_user_id):
        return jsonify({'error': 'Access denied'}), 403
    
    try:
        # 한국어 분석 수행
        analysis_result = process_korean_document(document.title, document.markdown_content)
        
        # 추가 분석 정보
        analysis_result.update({
            'document_id': document_id,
            'document_title': document.title,
            'document_language': analysis_result['language'],
            'analysis_timestamp': datetime.utcnow().isoformat(),
            'word_count': len(analysis_result['content_tokens']),
            'unique_words': len(set(analysis_result['content_tokens'])),
            'keyword_density': len(analysis_result['keywords']) / max(len(analysis_result['content_tokens']), 1)
        })
        
        return jsonify({
            'analysis': analysis_result,
            'recommendations': _generate_korean_recommendations(analysis_result)
        })
        
    except Exception as e:
        current_app.logger.error(f"Korean analysis failed for document {document_id}: {str(e)}")
        return jsonify({'error': 'Analysis failed', 'details': str(e)}), 500

def _generate_korean_recommendations(analysis: dict) -> dict:
    """한국어 분석 결과를 바탕으로 추천사항 생성"""
    recommendations = {
        'suggested_tags': [],
        'readability_score': 'good',  # 간단한 평가
        'improvements': []
    }
    
    # 키워드 기반 태그 추천
    if analysis.get('keywords'):
        top_keywords = sorted(analysis['keywords'], key=lambda x: x['count'], reverse=True)[:5]
        recommendations['suggested_tags'] = [kw['word'] for kw in top_keywords if kw['pos'] in ['NNG', 'NNP']]
    
    # 가독성 평가 (간단한 휴리스틱)
    if analysis.get('content_tokens'):
        avg_sentence_length = len(analysis['content_tokens']) / max(analysis.get('title_tokens', []), 1)
        if avg_sentence_length > 50:
            recommendations['readability_score'] = 'complex'
            recommendations['improvements'].append('문장을 더 짧게 나누는 것을 고려해보세요.')
        elif avg_sentence_length < 10:
            recommendations['readability_score'] = 'simple'
    
    # 키워드 밀도 평가
    keyword_density = analysis.get('keyword_density', 0)
    if keyword_density < 0.05:
        recommendations['improvements'].append('주요 키워드를 더 많이 사용하여 내용의 일관성을 높여보세요.')
    elif keyword_density > 0.15:
        recommendations['improvements'].append('키워드 사용을 줄여 자연스러운 글쓰기를 고려해보세요.')
    
    return recommendations

@korean_search_bp.route('/search/statistics', methods=['GET'])
@jwt_required_optional
@rate_limit_api("10 per minute")
@validate_request_security
def get_korean_search_statistics():
    """한국어 검색 통계"""
    current_user_id = get_jwt_identity()
    
    try:
        # OpenSearch 통계 (사용 가능한 경우)
        opensearch_service = get_opensearch_service()
        if opensearch_service:
            stats = opensearch_service.get_document_statistics()
            stats['source'] = 'opensearch'
            return jsonify({'statistics': stats})
        
        # PostgreSQL 기반 통계
        from app.models.tag import Tag
        from sqlalchemy import func
        
        # 기본 통계
        total_docs = Document.query.count()
        public_docs = Document.query.filter_by(is_public=True).count()
        
        if current_user_id:
            user_docs = Document.query.filter_by(user_id=current_user_id).count()
        else:
            user_docs = 0
        
        # 언어별 분포 (간단한 추정)
        korean_docs = Document.query.filter(
            Document.markdown_content.op('~*')('[가-힣]')
        ).count()
        
        # 인기 태그
        popular_tags = Tag.query.order_by(Tag.usage_count.desc()).limit(10).all()
        
        # 월별 생성 분포
        monthly_stats = Document.query.with_entities(
            func.date_trunc('month', Document.created_at).label('month'),
            func.count(Document.id).label('count')
        ).group_by('month').order_by('month').limit(12).all()
        
        stats = {
            'total_documents': total_docs,
            'public_documents': public_docs,
            'user_documents': user_docs,
            'estimated_korean_documents': korean_docs,
            'popular_tags': [(tag.name, tag.usage_count) for tag in popular_tags],
            'monthly_distribution': [(str(month), count) for month, count in monthly_stats],
            'source': 'postgresql'
        }
        
        return jsonify({'statistics': stats})
        
    except Exception as e:
        current_app.logger.error(f"Statistics query failed: {str(e)}")
        return jsonify({'error': 'Failed to get statistics', 'details': str(e)}), 500

@korean_search_bp.route('/search/health', methods=['GET'])
@jwt_required_optional
@rate_limit_api("5 per minute")
def get_search_health():
    """검색 시스템 상태 확인"""
    health_info = {
        'postgresql': {'status': 'available'},
        'opensearch': {'status': 'unavailable'},
        'korean_processor': {'status': 'unknown'}
    }
    
    try:
        # PostgreSQL 테스트
        Document.query.limit(1).first()
        health_info['postgresql']['status'] = 'healthy'
    except Exception as e:
        health_info['postgresql']['status'] = 'error'
        health_info['postgresql']['error'] = str(e)
    
    try:
        # OpenSearch 테스트
        opensearch_service = get_opensearch_service()
        if opensearch_service:
            opensearch_health = opensearch_service.health_check()
            health_info['opensearch'] = opensearch_health
        else:
            health_info['opensearch']['status'] = 'not_configured'
    except Exception as e:
        health_info['opensearch']['status'] = 'error'
        health_info['opensearch']['error'] = str(e)
    
    try:
        # 한국어 프로세서 테스트
        test_text = "안녕하세요. 테스트입니다."
        tokens = korean_processor.tokenize(test_text)
        health_info['korean_processor'] = {
            'status': 'healthy',
            'analyzer': korean_processor.analyzer_name,
            'test_tokens': len(tokens)
        }
    except Exception as e:
        health_info['korean_processor'] = {
            'status': 'error',
            'error': str(e)
        }
    
    # 전체 상태 결정
    overall_status = 'healthy'
    if health_info['postgresql']['status'] != 'healthy':
        overall_status = 'degraded'
    if health_info['korean_processor']['status'] != 'healthy':
        overall_status = 'degraded'
    
    return jsonify({
        'overall_status': overall_status,
        'components': health_info,
        'timestamp': datetime.utcnow().isoformat()
    })