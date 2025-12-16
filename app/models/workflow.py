from app import db
from datetime import datetime, timezone
from enum import Enum
from sqlalchemy import Index


def utc_now():
    """Return current UTC time as timezone-aware datetime."""
    return datetime.now(timezone.utc)


class WorkflowStatus(Enum):
    DRAFT = "draft"
    PENDING_REVIEW = "pending_review"
    IN_REVIEW = "in_review"
    APPROVED = "approved"
    REJECTED = "rejected"
    PUBLISHED = "published"
    ARCHIVED = "archived"

class WorkflowAction(Enum):
    SUBMIT_FOR_REVIEW = "submit_for_review"
    START_REVIEW = "start_review"
    APPROVE = "approve"
    REJECT = "reject"
    REQUEST_CHANGES = "request_changes"
    PUBLISH = "publish"
    ARCHIVE = "archive"
    WITHDRAW = "withdraw"

class DocumentWorkflow(db.Model):
    __tablename__ = 'document_workflows'
    
    id = db.Column(db.Integer, primary_key=True)
    document_id = db.Column(db.Integer, db.ForeignKey('documents.id'), nullable=False)
    
    # Workflow settings
    workflow_template_id = db.Column(db.Integer, db.ForeignKey('workflow_templates.id'), nullable=True)
    current_status = db.Column(db.Enum(WorkflowStatus), default=WorkflowStatus.DRAFT, nullable=False)
    requires_approval = db.Column(db.Boolean, default=False, nullable=False)
    
    # Current assignment
    assigned_reviewer_id = db.Column(db.Integer, db.ForeignKey('users.id'), nullable=True)
    assigned_at = db.Column(db.DateTime, nullable=True)
    due_date = db.Column(db.DateTime, nullable=True)
    
    # Metadata
    created_at = db.Column(db.DateTime, default=utc_now, nullable=False)
    updated_at = db.Column(db.DateTime, default=utc_now, onupdate=utc_now, nullable=False)
    
    # Relationships
    document = db.relationship('Document', backref='workflow')
    template = db.relationship('WorkflowTemplate', backref='workflows')
    assigned_reviewer = db.relationship('User', foreign_keys=[assigned_reviewer_id])
    
    # Indexes for performance
    __table_args__ = (
        Index('idx_workflow_document', 'document_id'),
        Index('idx_workflow_status', 'current_status'),
        Index('idx_workflow_reviewer', 'assigned_reviewer_id'),
        Index('idx_workflow_due_date', 'due_date'),
    )
    
    def can_perform_action(self, action, user_id):
        """Check if user can perform the given action on this workflow"""
        from app.models.user import User
        user = User.query.get(user_id)
        
        if not user:
            return False
        
        # Document author can always withdraw or submit for review
        if action in [WorkflowAction.WITHDRAW, WorkflowAction.SUBMIT_FOR_REVIEW]:
            return self.document.author_id == user_id
        
        # Admins can perform any action
        if user.is_admin:
            return True
        
        # Assigned reviewer can review
        if action in [WorkflowAction.START_REVIEW, WorkflowAction.APPROVE, WorkflowAction.REJECT, WorkflowAction.REQUEST_CHANGES]:
            return self.assigned_reviewer_id == user_id
        
        # Publisher role for publishing (simplified - in practice you'd have role management)
        if action == WorkflowAction.PUBLISH:
            return user.is_admin or self.assigned_reviewer_id == user_id
        
        return False
    
    def get_available_actions(self, user_id):
        """Get list of actions available to user for current status"""
        actions = []
        
        status_action_map = {
            WorkflowStatus.DRAFT: [WorkflowAction.SUBMIT_FOR_REVIEW],
            WorkflowStatus.PENDING_REVIEW: [WorkflowAction.START_REVIEW, WorkflowAction.WITHDRAW],
            WorkflowStatus.IN_REVIEW: [WorkflowAction.APPROVE, WorkflowAction.REJECT, WorkflowAction.REQUEST_CHANGES],
            WorkflowStatus.APPROVED: [WorkflowAction.PUBLISH],
            WorkflowStatus.REJECTED: [WorkflowAction.SUBMIT_FOR_REVIEW],
            WorkflowStatus.PUBLISHED: [WorkflowAction.ARCHIVE],
            WorkflowStatus.ARCHIVED: []
        }
        
        possible_actions = status_action_map.get(self.current_status, [])
        
        for action in possible_actions:
            if self.can_perform_action(action, user_id):
                actions.append(action.value)
        
        return actions
    
    def perform_action(self, action, user_id, comment=None):
        """Perform workflow action and update status"""
        from app.models.user import User
        
        if not self.can_perform_action(action, user_id):
            raise ValueError(f"User {user_id} cannot perform action {action.value}")
        
        old_status = self.current_status
        user = User.query.get(user_id)
        
        # Log the action
        workflow_log = WorkflowLog(
            workflow_id=self.id,
            action=action,
            performed_by_id=user_id,
            from_status=old_status,
            comment=comment
        )
        
        # Update status based on action
        if action == WorkflowAction.SUBMIT_FOR_REVIEW:
            self.current_status = WorkflowStatus.PENDING_REVIEW
            # Auto-assign to next available reviewer if template exists
            if self.template:
                next_reviewer = self.template.get_next_reviewer()
                if next_reviewer:
                    self.assigned_reviewer_id = next_reviewer.id
                    self.assigned_at = datetime.now(timezone.utc)
                    if self.template.review_days:
                        from datetime import timedelta
                        self.due_date = datetime.now(timezone.utc) + timedelta(days=self.template.review_days)
        
        elif action == WorkflowAction.START_REVIEW:
            self.current_status = WorkflowStatus.IN_REVIEW
        
        elif action == WorkflowAction.APPROVE:
            self.current_status = WorkflowStatus.APPROVED
        
        elif action == WorkflowAction.REJECT:
            self.current_status = WorkflowStatus.REJECTED
            self.assigned_reviewer_id = None
            self.assigned_at = None
            self.due_date = None
        
        elif action == WorkflowAction.REQUEST_CHANGES:
            self.current_status = WorkflowStatus.DRAFT
            self.assigned_reviewer_id = None
            self.assigned_at = None
            self.due_date = None
        
        elif action == WorkflowAction.PUBLISH:
            self.current_status = WorkflowStatus.PUBLISHED
            # Mark document as published
            self.document.is_published = True
            self.document.published_at = datetime.now(timezone.utc)
        
        elif action == WorkflowAction.ARCHIVE:
            self.current_status = WorkflowStatus.ARCHIVED
        
        elif action == WorkflowAction.WITHDRAW:
            self.current_status = WorkflowStatus.DRAFT
            self.assigned_reviewer_id = None
            self.assigned_at = None
            self.due_date = None
        
        workflow_log.to_status = self.current_status
        self.updated_at = datetime.now(timezone.utc)
        
        db.session.add(workflow_log)
        db.session.commit()
        
        # Send notifications
        self._send_workflow_notifications(action, user, old_status)
        
        return True
    
    def _send_workflow_notifications(self, action, actor, old_status):
        """Send notifications for workflow state changes"""
        # Implementation would send notifications to relevant users
        # For brevity, just logging key events
        
        notifications_to_send = []
        
        if action == WorkflowAction.SUBMIT_FOR_REVIEW and self.assigned_reviewer_id:
            # Notify assigned reviewer
            notifications_to_send.append({
                'user_id': self.assigned_reviewer_id,
                'title': f"Document review assigned: '{self.document.title}'",
                'message': f"{actor.username} submitted a document for your review"
            })
        
        elif action == WorkflowAction.APPROVE:
            # Notify document author
            notifications_to_send.append({
                'user_id': self.document.author_id,
                'title': f"Document approved: '{self.document.title}'",
                'message': f"{actor.username} approved your document"
            })
        
        elif action == WorkflowAction.REJECT:
            # Notify document author
            notifications_to_send.append({
                'user_id': self.document.author_id,
                'title': f"Document rejected: '{self.document.title}'",
                'message': f"{actor.username} rejected your document"
            })
        
        # Send notifications
        for notification_data in notifications_to_send:
            from app.models.notification import Notification, NotificationType
            Notification.create_notification(
                user_id=notification_data['user_id'],
                notification_type=NotificationType.DOCUMENT_UPDATED,
                title=notification_data['title'],
                message=notification_data['message'],
                document_id=self.document_id,
                actor_id=actor.id
            )
    
    def to_dict(self, include_logs=False):
        """Convert workflow to dictionary"""
        data = {
            'id': self.id,
            'document_id': self.document_id,
            'workflow_template_id': self.workflow_template_id,
            'current_status': self.current_status.value,
            'requires_approval': self.requires_approval,
            'assigned_reviewer_id': self.assigned_reviewer_id,
            'assigned_reviewer': self.assigned_reviewer.to_dict() if self.assigned_reviewer else None,
            'assigned_at': self.assigned_at.isoformat() if self.assigned_at else None,
            'due_date': self.due_date.isoformat() if self.due_date else None,
            'created_at': self.created_at.isoformat(),
            'updated_at': self.updated_at.isoformat()
        }
        
        if include_logs:
            data['logs'] = [log.to_dict() for log in self.logs]
        
        return data

class WorkflowTemplate(db.Model):
    __tablename__ = 'workflow_templates'
    
    id = db.Column(db.Integer, primary_key=True)
    name = db.Column(db.String(200), nullable=False)
    description = db.Column(db.Text)
    
    # Template settings
    requires_approval = db.Column(db.Boolean, default=True, nullable=False)
    review_days = db.Column(db.Integer, default=7)  # Days to complete review
    auto_publish = db.Column(db.Boolean, default=False, nullable=False)
    
    # Reviewers (simple implementation - could be extended with roles)
    reviewer_ids = db.Column(db.JSON)  # List of user IDs who can review
    
    # Metadata
    created_by_id = db.Column(db.Integer, db.ForeignKey('users.id'), nullable=False)
    is_active = db.Column(db.Boolean, default=True, nullable=False)
    created_at = db.Column(db.DateTime, default=utc_now, nullable=False)
    updated_at = db.Column(db.DateTime, default=utc_now, onupdate=utc_now, nullable=False)
    
    # Relationships
    created_by = db.relationship('User', foreign_keys=[created_by_id])
    
    def get_next_reviewer(self):
        """Get next available reviewer (simple round-robin)"""
        if not self.reviewer_ids:
            return None
        
        from app.models.user import User
        # Simple implementation - just return first active reviewer
        for reviewer_id in self.reviewer_ids:
            reviewer = User.query.filter_by(id=reviewer_id, is_active=True).first()
            if reviewer:
                return reviewer
        
        return None
    
    def to_dict(self):
        """Convert template to dictionary"""
        return {
            'id': self.id,
            'name': self.name,
            'description': self.description,
            'requires_approval': self.requires_approval,
            'review_days': self.review_days,
            'auto_publish': self.auto_publish,
            'reviewer_ids': self.reviewer_ids,
            'created_by_id': self.created_by_id,
            'created_by': self.created_by.to_dict() if self.created_by else None,
            'is_active': self.is_active,
            'created_at': self.created_at.isoformat(),
            'updated_at': self.updated_at.isoformat()
        }

class WorkflowLog(db.Model):
    __tablename__ = 'workflow_logs'
    
    id = db.Column(db.Integer, primary_key=True)
    workflow_id = db.Column(db.Integer, db.ForeignKey('document_workflows.id'), nullable=False)
    
    # Action details
    action = db.Column(db.Enum(WorkflowAction), nullable=False)
    performed_by_id = db.Column(db.Integer, db.ForeignKey('users.id'), nullable=False)
    from_status = db.Column(db.Enum(WorkflowStatus), nullable=True)
    to_status = db.Column(db.Enum(WorkflowStatus), nullable=True)
    comment = db.Column(db.Text)
    
    # Timestamp
    created_at = db.Column(db.DateTime, default=utc_now, nullable=False)
    
    # Relationships
    workflow = db.relationship('DocumentWorkflow', backref='logs')
    performed_by = db.relationship('User', foreign_keys=[performed_by_id])
    
    # Indexes
    __table_args__ = (
        Index('idx_workflow_log_workflow', 'workflow_id'),
        Index('idx_workflow_log_created', 'created_at'),
    )
    
    def to_dict(self):
        """Convert log entry to dictionary"""
        return {
            'id': self.id,
            'workflow_id': self.workflow_id,
            'action': self.action.value,
            'performed_by': self.performed_by.to_dict() if self.performed_by else None,
            'from_status': self.from_status.value if self.from_status else None,
            'to_status': self.to_status.value if self.to_status else None,
            'comment': self.comment,
            'created_at': self.created_at.isoformat()
        }