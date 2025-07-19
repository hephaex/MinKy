from flask import Blueprint, request, jsonify, current_app
from flask_jwt_extended import jwt_required, get_jwt_identity
from app.models.document import Document
from app.models.user import User
from app.models.workflow import DocumentWorkflow, WorkflowTemplate, WorkflowAction, WorkflowStatus
from app.middleware.security import rate_limit_api, validate_request_security, audit_log
from marshmallow import Schema, fields, ValidationError
from app import db
from datetime import datetime, timedelta

workflows_bp = Blueprint('workflows', __name__)

class WorkflowActionSchema(Schema):
    action = fields.Str(required=True)
    comment = fields.Str(allow_none=True)

class WorkflowTemplateSchema(Schema):
    name = fields.Str(required=True, validate=lambda x: len(x) >= 3)
    description = fields.Str(allow_none=True)
    requires_approval = fields.Bool(load_default=True)
    review_days = fields.Int(load_default=7, validate=lambda x: 1 <= x <= 30)
    auto_publish = fields.Bool(load_default=False)
    reviewer_ids = fields.List(fields.Int(), load_default=[])

@workflows_bp.route('/documents/<int:document_id>/workflow', methods=['GET'])
@jwt_required()
@rate_limit_api("50 per minute")
@validate_request_security
@audit_log("view_document_workflow")
def get_document_workflow(document_id):
    """Get workflow information for a document"""
    current_user_id = get_jwt_identity()
    user = User.query.get(current_user_id)
    
    if not user:
        return jsonify({'error': 'User not found'}), 404
    
    document = Document.query.get_or_404(document_id)
    
    # Check if user can view this document
    if not document.can_view(current_user_id):
        return jsonify({'error': 'Access denied'}), 403
    
    try:
        workflow = DocumentWorkflow.query.filter_by(document_id=document_id).first()
        
        if not workflow:
            # Create default workflow for document
            workflow = DocumentWorkflow(
                document_id=document_id,
                current_status=WorkflowStatus.DRAFT,
                requires_approval=False
            )
            db.session.add(workflow)
            db.session.commit()
        
        # Get available actions for current user
        available_actions = workflow.get_available_actions(current_user_id)
        
        workflow_data = workflow.to_dict(include_logs=True)
        workflow_data['available_actions'] = available_actions
        
        return jsonify({
            'workflow': workflow_data,
            'document': {
                'id': document.id,
                'title': document.title,
                'author_id': document.user_id
            }
        })
        
    except Exception as e:
        current_app.logger.error(f"Error getting document workflow: {str(e)}")
        return jsonify({'error': 'Failed to get workflow information'}), 500

@workflows_bp.route('/documents/<int:document_id>/workflow/action', methods=['POST'])
@jwt_required()
@rate_limit_api("20 per minute")
@validate_request_security
@audit_log("perform_workflow_action")
def perform_workflow_action(document_id):
    """Perform an action on a document workflow"""
    current_user_id = get_jwt_identity()
    user = User.query.get(current_user_id)
    
    if not user:
        return jsonify({'error': 'User not found'}), 404
    
    document = Document.query.get_or_404(document_id)
    
    data = request.get_json()
    if not data:
        return jsonify({'error': 'Request body required'}), 400
    
    # Validate data
    schema = WorkflowActionSchema()
    try:
        validated_data = schema.load(data)
    except ValidationError as e:
        return jsonify({'error': 'Invalid data', 'details': e.messages}), 400
    
    action_str = validated_data['action']
    comment = validated_data.get('comment')
    
    # Convert string to enum
    try:
        action = WorkflowAction(action_str)
    except ValueError:
        return jsonify({'error': f'Invalid action: {action_str}'}), 400
    
    try:
        workflow = DocumentWorkflow.query.filter_by(document_id=document_id).first()
        
        if not workflow:
            # Create workflow if it doesn't exist
            workflow = DocumentWorkflow(
                document_id=document_id,
                current_status=WorkflowStatus.DRAFT,
                requires_approval=True
            )
            db.session.add(workflow)
            db.session.commit()
        
        # Perform the action
        workflow.perform_action(action, current_user_id, comment)
        
        # Get updated workflow with available actions
        available_actions = workflow.get_available_actions(current_user_id)
        workflow_data = workflow.to_dict()
        workflow_data['available_actions'] = available_actions
        
        return jsonify({
            'message': f'Action {action_str} performed successfully',
            'workflow': workflow_data
        })
        
    except ValueError as e:
        return jsonify({'error': str(e)}), 403
    except Exception as e:
        current_app.logger.error(f"Error performing workflow action: {str(e)}")
        return jsonify({'error': 'Failed to perform workflow action'}), 500

@workflows_bp.route('/workflows/pending', methods=['GET'])
@jwt_required()
@rate_limit_api("30 per minute")
@validate_request_security
@audit_log("view_pending_workflows")
def get_pending_workflows():
    """Get workflows pending review for the current user"""
    current_user_id = get_jwt_identity()
    user = User.query.get(current_user_id)
    
    if not user:
        return jsonify({'error': 'User not found'}), 404
    
    try:
        # Get workflows assigned to current user for review
        pending_workflows = DocumentWorkflow.query.filter(
            DocumentWorkflow.assigned_reviewer_id == current_user_id,
            DocumentWorkflow.current_status.in_([
                WorkflowStatus.PENDING_REVIEW,
                WorkflowStatus.IN_REVIEW
            ])
        ).join(Document).order_by(DocumentWorkflow.assigned_at.asc()).all()
        
        # Get workflows where user is author and need attention
        author_workflows = DocumentWorkflow.query.filter(
            DocumentWorkflow.current_status.in_([
                WorkflowStatus.REJECTED,
                WorkflowStatus.APPROVED
            ])
        ).join(Document).filter(
            Document.user_id == current_user_id
        ).order_by(DocumentWorkflow.updated_at.desc()).limit(10).all()
        
        workflows_data = []
        for workflow in pending_workflows:
            workflow_dict = workflow.to_dict()
            workflow_dict['document'] = workflow.document.to_dict()
            workflow_dict['available_actions'] = workflow.get_available_actions(current_user_id)
            workflow_dict['priority'] = 'high' if workflow.due_date and workflow.due_date < datetime.utcnow() else 'normal'
            workflows_data.append(workflow_dict)
        
        author_workflows_data = []
        for workflow in author_workflows:
            workflow_dict = workflow.to_dict()
            workflow_dict['document'] = {
                'id': workflow.document.id,
                'title': workflow.document.title
            }
            workflow_dict['available_actions'] = workflow.get_available_actions(current_user_id)
            author_workflows_data.append(workflow_dict)
        
        return jsonify({
            'pending_reviews': workflows_data,
            'my_documents': author_workflows_data,
            'summary': {
                'pending_reviews_count': len(workflows_data),
                'overdue_count': len([w for w in workflows_data if w.get('priority') == 'high']),
                'my_documents_count': len(author_workflows_data)
            }
        })
        
    except Exception as e:
        current_app.logger.error(f"Error getting pending workflows: {str(e)}")
        return jsonify({'error': 'Failed to get pending workflows'}), 500

@workflows_bp.route('/workflow-templates', methods=['GET'])
@jwt_required()
@rate_limit_api("50 per minute")
@validate_request_security
@audit_log("view_workflow_templates")
def get_workflow_templates():
    """Get available workflow templates"""
    current_user_id = get_jwt_identity()
    user = User.query.get(current_user_id)
    
    if not user:
        return jsonify({'error': 'User not found'}), 404
    
    try:
        templates = WorkflowTemplate.query.filter_by(is_active=True).order_by(
            WorkflowTemplate.name
        ).all()
        
        return jsonify({
            'templates': [template.to_dict() for template in templates]
        })
        
    except Exception as e:
        current_app.logger.error(f"Error getting workflow templates: {str(e)}")
        return jsonify({'error': 'Failed to get workflow templates'}), 500

@workflows_bp.route('/workflow-templates', methods=['POST'])
@jwt_required()
@rate_limit_api("10 per minute")
@validate_request_security
@audit_log("create_workflow_template")
def create_workflow_template():
    """Create a new workflow template (admin only)"""
    current_user_id = get_jwt_identity()
    user = User.query.get(current_user_id)
    
    if not user or not user.is_admin:
        return jsonify({'error': 'Admin privileges required'}), 403
    
    data = request.get_json()
    if not data:
        return jsonify({'error': 'Request body required'}), 400
    
    # Validate data
    schema = WorkflowTemplateSchema()
    try:
        validated_data = schema.load(data)
    except ValidationError as e:
        return jsonify({'error': 'Invalid data', 'details': e.messages}), 400
    
    try:
        # Validate reviewer IDs
        reviewer_ids = validated_data.get('reviewer_ids', [])
        if reviewer_ids:
            valid_reviewers = User.query.filter(
                User.id.in_(reviewer_ids),
                User.is_active == True
            ).count()
            if valid_reviewers != len(reviewer_ids):
                return jsonify({'error': 'Some reviewer IDs are invalid'}), 400
        
        template = WorkflowTemplate(
            name=validated_data['name'],
            description=validated_data.get('description'),
            requires_approval=validated_data['requires_approval'],
            review_days=validated_data['review_days'],
            auto_publish=validated_data['auto_publish'],
            reviewer_ids=reviewer_ids,
            created_by_id=current_user_id
        )
        
        db.session.add(template)
        db.session.commit()
        
        return jsonify({
            'message': 'Workflow template created successfully',
            'template': template.to_dict()
        }), 201
        
    except Exception as e:
        current_app.logger.error(f"Error creating workflow template: {str(e)}")
        return jsonify({'error': 'Failed to create workflow template'}), 500

@workflows_bp.route('/workflow-templates/<int:template_id>', methods=['PUT'])
@jwt_required()
@rate_limit_api("10 per minute")
@validate_request_security
@audit_log("update_workflow_template")
def update_workflow_template(template_id):
    """Update a workflow template (admin only)"""
    current_user_id = get_jwt_identity()
    user = User.query.get(current_user_id)
    
    if not user or not user.is_admin:
        return jsonify({'error': 'Admin privileges required'}), 403
    
    template = WorkflowTemplate.query.get_or_404(template_id)
    
    data = request.get_json()
    if not data:
        return jsonify({'error': 'Request body required'}), 400
    
    # Validate data
    schema = WorkflowTemplateSchema()
    try:
        validated_data = schema.load(data, partial=True)
    except ValidationError as e:
        return jsonify({'error': 'Invalid data', 'details': e.messages}), 400
    
    try:
        # Validate reviewer IDs if provided
        if 'reviewer_ids' in validated_data:
            reviewer_ids = validated_data['reviewer_ids']
            if reviewer_ids:
                valid_reviewers = User.query.filter(
                    User.id.in_(reviewer_ids),
                    User.is_active == True
                ).count()
                if valid_reviewers != len(reviewer_ids):
                    return jsonify({'error': 'Some reviewer IDs are invalid'}), 400
        
        # Update template
        for key, value in validated_data.items():
            setattr(template, key, value)
        
        template.updated_at = datetime.utcnow()
        db.session.commit()
        
        return jsonify({
            'message': 'Workflow template updated successfully',
            'template': template.to_dict()
        })
        
    except Exception as e:
        current_app.logger.error(f"Error updating workflow template: {str(e)}")
        return jsonify({'error': 'Failed to update workflow template'}), 500

@workflows_bp.route('/documents/<int:document_id>/workflow/assign-template/<int:template_id>', methods=['POST'])
@jwt_required()
@rate_limit_api("20 per minute")
@validate_request_security
@audit_log("assign_workflow_template")
def assign_workflow_template(document_id, template_id):
    """Assign a workflow template to a document"""
    current_user_id = get_jwt_identity()
    user = User.query.get(current_user_id)
    
    if not user:
        return jsonify({'error': 'User not found'}), 404
    
    document = Document.query.get_or_404(document_id)
    template = WorkflowTemplate.query.get_or_404(template_id)
    
    # Check if user can edit this document
    if not document.can_edit(current_user_id):
        return jsonify({'error': 'Access denied'}), 403
    
    try:
        workflow = DocumentWorkflow.query.filter_by(document_id=document_id).first()
        
        if not workflow:
            workflow = DocumentWorkflow(
                document_id=document_id,
                current_status=WorkflowStatus.DRAFT
            )
            db.session.add(workflow)
        
        # Can only assign template if workflow is in draft status
        if workflow.current_status != WorkflowStatus.DRAFT:
            return jsonify({'error': 'Cannot assign template to document not in draft status'}), 400
        
        workflow.workflow_template_id = template_id
        workflow.requires_approval = template.requires_approval
        
        # Auto-assign first reviewer if template has reviewers
        if template.reviewer_ids:
            next_reviewer = template.get_next_reviewer()
            if next_reviewer:
                workflow.assigned_reviewer_id = next_reviewer.id
        
        db.session.commit()
        
        return jsonify({
            'message': 'Workflow template assigned successfully',
            'workflow': workflow.to_dict(),
            'template': template.to_dict()
        })
        
    except Exception as e:
        current_app.logger.error(f"Error assigning workflow template: {str(e)}")
        return jsonify({'error': 'Failed to assign workflow template'}), 500

@workflows_bp.route('/workflows/stats', methods=['GET'])
@jwt_required()
@rate_limit_api("20 per minute")
@validate_request_security
@audit_log("view_workflow_stats")
def get_workflow_stats():
    """Get workflow statistics"""
    current_user_id = get_jwt_identity()
    user = User.query.get(current_user_id)
    
    if not user:
        return jsonify({'error': 'User not found'}), 404
    
    try:
        # Get stats for current user
        stats = {}
        
        # Documents I'm reviewing
        pending_reviews = DocumentWorkflow.query.filter(
            DocumentWorkflow.assigned_reviewer_id == current_user_id,
            DocumentWorkflow.current_status.in_([
                WorkflowStatus.PENDING_REVIEW,
                WorkflowStatus.IN_REVIEW
            ])
        ).count()
        
        overdue_reviews = DocumentWorkflow.query.filter(
            DocumentWorkflow.assigned_reviewer_id == current_user_id,
            DocumentWorkflow.current_status.in_([
                WorkflowStatus.PENDING_REVIEW,
                WorkflowStatus.IN_REVIEW
            ]),
            DocumentWorkflow.due_date < datetime.utcnow()
        ).count()
        
        # My documents in workflow
        my_documents_in_workflow = DocumentWorkflow.query.join(Document).filter(
            Document.user_id == current_user_id,
            DocumentWorkflow.current_status != WorkflowStatus.DRAFT
        ).count()
        
        # My published documents
        my_published_documents = DocumentWorkflow.query.join(Document).filter(
            Document.user_id == current_user_id,
            DocumentWorkflow.current_status == WorkflowStatus.PUBLISHED
        ).count()
        
        stats = {
            'pending_reviews': pending_reviews,
            'overdue_reviews': overdue_reviews,
            'my_documents_in_workflow': my_documents_in_workflow,
            'my_published_documents': my_published_documents
        }
        
        # Add system-wide stats for admins
        if user.is_admin:
            total_workflows = DocumentWorkflow.query.count()
            active_workflows = DocumentWorkflow.query.filter(
                DocumentWorkflow.current_status.in_([
                    WorkflowStatus.PENDING_REVIEW,
                    WorkflowStatus.IN_REVIEW,
                    WorkflowStatus.APPROVED
                ])
            ).count()
            
            stats['system'] = {
                'total_workflows': total_workflows,
                'active_workflows': active_workflows,
                'total_templates': WorkflowTemplate.query.filter_by(is_active=True).count()
            }
        
        return jsonify({'stats': stats})
        
    except Exception as e:
        current_app.logger.error(f"Error getting workflow stats: {str(e)}")
        return jsonify({'error': 'Failed to get workflow statistics'}), 500