"""Add category_id to documents

Revision ID: a1b2c3d4e5f6
Revises: c6306320dba4
Create Date: 2026-02-06

"""
from alembic import op
import sqlalchemy as sa


# revision identifiers, used by Alembic.
revision = 'a1b2c3d4e5f6'
down_revision = 'c6306320dba4'
branch_labels = None
depends_on = None


def upgrade():
    with op.batch_alter_table('documents', schema=None) as batch_op:
        batch_op.add_column(sa.Column('category_id', sa.Integer(), nullable=True))
        batch_op.create_foreign_key(
            'fk_documents_category_id',
            'categories',
            ['category_id'],
            ['id']
        )


def downgrade():
    with op.batch_alter_table('documents', schema=None) as batch_op:
        batch_op.drop_constraint('fk_documents_category_id', type_='foreignkey')
        batch_op.drop_column('category_id')
