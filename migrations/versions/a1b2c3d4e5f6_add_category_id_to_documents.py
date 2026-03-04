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
    # Create categories table if it doesn't exist
    op.create_table('categories',
        sa.Column('id', sa.Integer(), nullable=False),
        sa.Column('name', sa.String(length=100), nullable=False),
        sa.Column('slug', sa.String(length=100), nullable=False),
        sa.Column('description', sa.Text(), nullable=True),
        sa.Column('color', sa.String(length=7), nullable=True),
        sa.Column('icon', sa.String(length=50), nullable=True),
        sa.Column('parent_id', sa.Integer(), nullable=True),
        sa.Column('sort_order', sa.Integer(), nullable=True, default=0),
        sa.Column('is_active', sa.Boolean(), nullable=True, default=True),
        sa.Column('created_by', sa.Integer(), nullable=True),
        sa.Column('created_at', sa.DateTime(), nullable=True),
        sa.Column('updated_at', sa.DateTime(), nullable=True),
        sa.ForeignKeyConstraint(['created_by'], ['users.id'], ),
        sa.ForeignKeyConstraint(['parent_id'], ['categories.id'], ),
        sa.PrimaryKeyConstraint('id'),
        sa.UniqueConstraint('slug')
    )
    op.create_index('idx_categories_parent', 'categories', ['parent_id'], unique=False)
    op.create_index('idx_categories_slug', 'categories', ['slug'], unique=False)

    # Add category_id to documents
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

    op.drop_index('idx_categories_slug', table_name='categories')
    op.drop_index('idx_categories_parent', table_name='categories')
    op.drop_table('categories')
