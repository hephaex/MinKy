"""Add performance indexes

Revision ID: b1c2d3e4f5g6
Revises: a1b2c3d4e5f6
Create Date: 2026-02-06

"""
from alembic import op

# revision identifiers, used by Alembic.
revision = 'b1c2d3e4f5g6'
down_revision = 'a1b2c3d4e5f6'
branch_labels = None
depends_on = None


def upgrade():
    # Documents table indexes for common query patterns
    with op.batch_alter_table('documents', schema=None) as batch_op:
        batch_op.create_index('idx_documents_user_id', ['user_id'], unique=False)
        batch_op.create_index('idx_documents_is_public', ['is_public'], unique=False)
        batch_op.create_index('idx_documents_created_at', ['created_at'], unique=False)
        batch_op.create_index('idx_documents_updated_at', ['updated_at'], unique=False)
        batch_op.create_index('idx_documents_category_id', ['category_id'], unique=False)
        # Composite index for visibility filtering
        batch_op.create_index(
            'idx_documents_visibility',
            ['is_public', 'user_id'],
            unique=False
        )

    # Document tags reverse lookup index
    with op.batch_alter_table('document_tags', schema=None) as batch_op:
        batch_op.create_index('idx_document_tags_tag_id', ['tag_id'], unique=False)

    # Comments table indexes
    with op.batch_alter_table('comments', schema=None) as batch_op:
        batch_op.create_index('idx_comments_document_id', ['document_id'], unique=False)
        batch_op.create_index('idx_comments_user_id', ['user_id'], unique=False)
        batch_op.create_index('idx_comments_parent_id', ['parent_id'], unique=False)

    # Document versions index for version history queries
    with op.batch_alter_table('document_versions', schema=None) as batch_op:
        batch_op.create_index(
            'idx_versions_document_version',
            ['document_id', 'version_number'],
            unique=False
        )

    # Tags created_by index
    with op.batch_alter_table('tags', schema=None) as batch_op:
        batch_op.create_index('idx_tags_created_by', ['created_by'], unique=False)

    # Attachments document_id index
    with op.batch_alter_table('attachments', schema=None) as batch_op:
        batch_op.create_index('idx_attachments_document_id', ['document_id'], unique=False)


def downgrade():
    # Remove indexes in reverse order
    with op.batch_alter_table('attachments', schema=None) as batch_op:
        batch_op.drop_index('idx_attachments_document_id')

    with op.batch_alter_table('tags', schema=None) as batch_op:
        batch_op.drop_index('idx_tags_created_by')

    with op.batch_alter_table('document_versions', schema=None) as batch_op:
        batch_op.drop_index('idx_versions_document_version')

    with op.batch_alter_table('comments', schema=None) as batch_op:
        batch_op.drop_index('idx_comments_parent_id')
        batch_op.drop_index('idx_comments_user_id')
        batch_op.drop_index('idx_comments_document_id')

    with op.batch_alter_table('document_tags', schema=None) as batch_op:
        batch_op.drop_index('idx_document_tags_tag_id')

    with op.batch_alter_table('documents', schema=None) as batch_op:
        batch_op.drop_index('idx_documents_visibility')
        batch_op.drop_index('idx_documents_category_id')
        batch_op.drop_index('idx_documents_updated_at')
        batch_op.drop_index('idx_documents_created_at')
        batch_op.drop_index('idx_documents_is_public')
        batch_op.drop_index('idx_documents_user_id')
