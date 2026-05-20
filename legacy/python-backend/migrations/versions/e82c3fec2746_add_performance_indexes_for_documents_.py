"""Add performance indexes for documents and tags

Revision ID: e82c3fec2746
Revises: b1c2d3e4f5g6
Create Date: 2026-02-12 10:29:49.338764

"""
from alembic import op
import sqlalchemy as sa
from sqlalchemy import inspect


# revision identifiers, used by Alembic.
revision = 'e82c3fec2746'
down_revision = 'b1c2d3e4f5g6'
branch_labels = None
depends_on = None


def index_exists(table_name, index_name):
    """Check if an index exists on a table."""
    connection = op.get_bind()
    inspector = inspect(connection)
    indexes = inspector.get_indexes(table_name)
    return any(idx['name'] == index_name for idx in indexes)


def upgrade():
    # Only create indexes that don't already exist (avoid duplicates from b1c2d3e4f5g6)

    # Documents indexes - only create new ones not in previous migration
    with op.batch_alter_table('documents', schema=None) as batch_op:
        # This is a new composite index (different from idx_documents_visibility)
        if not index_exists('documents', 'idx_documents_user_visibility'):
            batch_op.create_index('idx_documents_user_visibility', ['user_id', 'is_public'], unique=False)

    # Tags indexes - only create new ones
    with op.batch_alter_table('tags', schema=None) as batch_op:
        if not index_exists('tags', 'idx_tags_created_at'):
            batch_op.create_index('idx_tags_created_at', ['created_at'], unique=False)


def downgrade():
    # Only drop indexes that were created by this migration
    with op.batch_alter_table('tags', schema=None) as batch_op:
        if index_exists('tags', 'idx_tags_created_at'):
            batch_op.drop_index('idx_tags_created_at')

    with op.batch_alter_table('documents', schema=None) as batch_op:
        if index_exists('documents', 'idx_documents_user_visibility'):
            batch_op.drop_index('idx_documents_user_visibility')
