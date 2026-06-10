#!/usr/bin/env python3
"""
Minky Flask→Rust data migration (Decision C = A1).

Source DB  : minky      (Flask schema, SELECT-only — never ALTER/DROP/UPDATE/DELETE)
Target DB  : minky_rust (Rust schema, additive columns from migration 011)

Usage:
    DRY_RUN=1  python scripts/migrate_flask_to_rust.py   # verify mapping, no writes
    python scripts/migrate_flask_to_rust.py               # execute migration

Both source and target DSN are read from environment:
    FLASK_DATABASE_URL  — e.g. postgresql://minky_app:pass@host:5432/minky
    RUST_DATABASE_URL   — e.g. postgresql://minky_app:pass@host:5432/minky_rust

Idempotent: ON CONFLICT DO NOTHING on every INSERT. Safe to re-run.
[CAS-deferred] Live execution requires CAS recovery (DB host 172.20.10.200 unreachable).
"""

from __future__ import annotations

import hashlib
import json
import os
import sys
import uuid
from typing import Any, Optional

# ─── Pure functions (tested without DB) ────────────────────────────────────

# Fixed namespace UUID for deterministic document ID mapping.
# Changing this would break idempotency — never modify after first migration.
MIGRATION_NAMESPACE = uuid.UUID("1d6b1000-6b40-5000-a000-000000000001")


def make_document_uuid(flask_id: int) -> uuid.UUID:
    """Convert Flask document INTEGER id to a deterministic UUIDv5.

    Same flask_id always → same UUID. Different flask_ids → different UUIDs.
    Used for every FK rewrite (document_tags, comments, document_versions).
    """
    return uuid.uuid5(MIGRATION_NAMESPACE, f"minky:document:{flask_id}")


def map_user_role(is_admin: bool) -> str:
    """Map Flask is_admin BOOLEAN to Rust user_role ENUM."""
    return "admin" if is_admin else "user"


def coerce_user_id(user_id: Optional[int], default_user_id: int) -> int:
    """Return user_id if set, else default_user_id.

    Flask allows NULL user_id (unauthenticated personal KB).
    Rust's users/documents/tags/categories have NOT NULL constraints.
    """
    return user_id if user_id is not None else default_user_id


def map_document_fields(row: dict[str, Any], default_user_id: int) -> dict[str, Any]:
    """Transform a Flask document row to Rust schema fields.

    Key mappings:
      Flask id (int)           → Rust id (UUID via make_document_uuid)
      Flask markdown_content   → Rust content
      Flask user_id (nullable) → Rust user_id NOT NULL (fallback to default_user_id)
      Flask author/html_content/document_metadata/is_published/published_at → additive columns
    """
    flask_id: int = row["id"]
    return {
        "id": make_document_uuid(flask_id),
        "title": row["title"],
        "content": row["markdown_content"],
        "category_id": row.get("category_id"),
        "user_id": coerce_user_id(row.get("user_id"), default_user_id),
        "is_public": row.get("is_public", True),
        "view_count": 0,
        "created_at": row.get("created_at"),
        "updated_at": row.get("updated_at"),
        # additive columns (from migration 011)
        "author": row.get("author"),
        "html_content": row.get("html_content"),
        "document_metadata": json.dumps(row["document_metadata"]) if row.get("document_metadata") else None,
        "is_published": row.get("is_published", False),
        "published_at": row.get("published_at"),
        "_flask_id": flask_id,
    }


def map_version_fields(row: dict[str, Any], default_user_id: int) -> dict[str, Any]:
    """Transform a Flask document_version row to Rust schema fields.

    Flask markdown_content → Rust content.
    Flask created_by (nullable) → Rust created_by NOT NULL.
    """
    return {
        "document_id": make_document_uuid(row["document_id"]),
        "content": row["markdown_content"],
        "version_number": row["version_number"],
        "created_by": coerce_user_id(row.get("created_by"), default_user_id),
        "created_at": row.get("created_at"),
        # additive
        "title": row.get("title"),
        "author": row.get("author"),
        "html_content": row.get("html_content"),
        "content_hash": row.get("content_hash"),
        "change_summary": row.get("change_summary"),
    }


def content_hash_sample(content: str) -> str:
    """SHA-256 hex digest for verification sampling."""
    return hashlib.sha256(content.encode("utf-8")).hexdigest()


# ─── DB-dependent functions (require live connection) ───────────────────────

def _connect(dsn: str):
    try:
        import psycopg2
        import psycopg2.extras
        conn = psycopg2.connect(dsn)
        conn.autocommit = False
        return conn
    except ImportError:
        sys.exit("psycopg2 not installed: pip install psycopg2-binary")


def _find_default_user_id(src_cur) -> int:
    """Return admin user id from source DB; fallback to first user."""
    src_cur.execute("SELECT id FROM users WHERE is_admin = true ORDER BY id LIMIT 1")
    row = src_cur.fetchone()
    if row:
        return row[0]
    src_cur.execute("SELECT id FROM users ORDER BY id LIMIT 1")
    row = src_cur.fetchone()
    if not row:
        raise RuntimeError("Source DB has no users — cannot determine default_user_id")
    return row[0]


def migrate_users(src_cur, dst_cur, dry_run: bool) -> int:
    """Migrate users (same INTEGER id, map is_admin → role enum)."""
    src_cur.execute(
        "SELECT id, email, username, password_hash, is_active, is_admin, "
        "       failed_login_attempts, locked_until, created_at, updated_at "
        "FROM users ORDER BY id"
    )
    rows = src_cur.fetchall()
    cols = [d[0] for d in src_cur.description]
    count = 0
    for raw in rows:
        row = dict(zip(cols, raw))
        role = map_user_role(row["is_admin"])
        if not dry_run:
            dst_cur.execute(
                """
                INSERT INTO users (id, email, username, password_hash, role, is_active,
                                   failed_login_attempts, locked_until, created_at, updated_at)
                VALUES (%s, %s, %s, %s, %s::user_role, %s, %s, %s, %s, %s)
                ON CONFLICT (id) DO NOTHING
                """,
                (
                    row["id"], row["email"], row["username"], row["password_hash"],
                    role, row["is_active"], row.get("failed_login_attempts", 0),
                    row.get("locked_until"), row["created_at"], row["updated_at"],
                ),
            )
        count += 1
    if not dry_run:
        dst_cur.execute("SELECT setval('users_id_seq', (SELECT MAX(id) FROM users))")
    return count


def migrate_categories(src_cur, dst_cur, default_user_id: int, dry_run: bool) -> int:
    src_cur.execute(
        "SELECT id, name, slug, description, parent_id, created_by, "
        "       color, sort_order, is_active, created_at, updated_at "
        "FROM categories ORDER BY id"
    )
    rows = src_cur.fetchall()
    cols = [d[0] for d in src_cur.description]
    count = 0
    for raw in rows:
        row = dict(zip(cols, raw))
        uid = coerce_user_id(row.get("created_by"), default_user_id)
        if not dry_run:
            dst_cur.execute(
                """
                INSERT INTO categories (id, name, parent_id, user_id, created_at, updated_at,
                                        slug, description, color, sort_order, is_active)
                VALUES (%s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s)
                ON CONFLICT (id) DO NOTHING
                """,
                (
                    row["id"], row["name"], row.get("parent_id"), uid,
                    row["created_at"], row["updated_at"],
                    row.get("slug"), row.get("description"),
                    row.get("color", "#007bff"), row.get("sort_order", 0),
                    row.get("is_active", True),
                ),
            )
        count += 1
    if not dry_run:
        dst_cur.execute("SELECT setval('categories_id_seq', (SELECT MAX(id) FROM categories))")
    return count


def migrate_tags(src_cur, dst_cur, default_user_id: int, dry_run: bool) -> int:
    src_cur.execute(
        "SELECT id, name, slug, description, color, created_by, created_at "
        "FROM tags ORDER BY id"
    )
    rows = src_cur.fetchall()
    cols = [d[0] for d in src_cur.description]
    count = 0
    for raw in rows:
        row = dict(zip(cols, raw))
        uid = coerce_user_id(row.get("created_by"), default_user_id)
        if not dry_run:
            dst_cur.execute(
                """
                INSERT INTO tags (id, name, user_id, created_at, slug, description, color)
                VALUES (%s, %s, %s, %s, %s, %s, %s)
                ON CONFLICT (id) DO NOTHING
                """,
                (
                    row["id"], row["name"], uid, row["created_at"],
                    row.get("slug"), row.get("description"), row.get("color", "#007bff"),
                ),
            )
        count += 1
    if not dry_run:
        dst_cur.execute("SELECT setval('tags_id_seq', (SELECT MAX(id) FROM tags))")
    return count


def migrate_documents(src_cur, dst_cur, default_user_id: int, dry_run: bool) -> int:
    src_cur.execute(
        "SELECT id, title, markdown_content, author, html_content, "
        "       user_id, category_id, is_public, is_published, published_at, "
        "       document_metadata, created_at, updated_at "
        "FROM documents ORDER BY id"
    )
    rows = src_cur.fetchall()
    cols = [d[0] for d in src_cur.description]
    count = 0
    for raw in rows:
        row = dict(zip(cols, raw))
        mapped = map_document_fields(row, default_user_id)
        flask_id = mapped.pop("_flask_id")
        if not dry_run:
            dst_cur.execute(
                """
                INSERT INTO documents (id, title, content, category_id, user_id, is_public,
                                       view_count, created_at, updated_at,
                                       author, html_content, document_metadata,
                                       is_published, published_at)
                VALUES (%s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s::jsonb, %s, %s)
                ON CONFLICT (id) DO NOTHING
                """,
                (
                    str(mapped["id"]), mapped["title"], mapped["content"],
                    mapped["category_id"], mapped["user_id"], mapped["is_public"],
                    mapped["view_count"], mapped["created_at"], mapped["updated_at"],
                    mapped["author"], mapped["html_content"], mapped["document_metadata"],
                    mapped["is_published"], mapped["published_at"],
                ),
            )
            dst_cur.execute(
                """
                INSERT INTO flask_document_id_mapping (flask_id, rust_id)
                VALUES (%s, %s)
                ON CONFLICT (flask_id) DO NOTHING
                """,
                (flask_id, str(mapped["id"])),
            )
        count += 1
    return count


def migrate_document_tags(src_cur, dst_cur, dry_run: bool) -> int:
    src_cur.execute("SELECT document_id, tag_id FROM document_tags")
    rows = src_cur.fetchall()
    count = 0
    for flask_doc_id, tag_id in rows:
        rust_doc_id = str(make_document_uuid(flask_doc_id))
        if not dry_run:
            dst_cur.execute(
                """
                INSERT INTO document_tags (document_id, tag_id)
                VALUES (%s, %s)
                ON CONFLICT (document_id, tag_id) DO NOTHING
                """,
                (rust_doc_id, tag_id),
            )
        count += 1
    return count


def migrate_comments(src_cur, dst_cur, default_user_id: int, dry_run: bool) -> int:
    src_cur.execute(
        "SELECT id, content, document_id, user_id, parent_id, created_at, updated_at "
        "FROM comments ORDER BY id"
    )
    rows = src_cur.fetchall()
    cols = [d[0] for d in src_cur.description]
    count = 0
    for raw in rows:
        row = dict(zip(cols, raw))
        rust_doc_id = str(make_document_uuid(row["document_id"]))
        uid = coerce_user_id(row.get("user_id"), default_user_id)
        if not dry_run:
            dst_cur.execute(
                """
                INSERT INTO comments (id, content, document_id, user_id, parent_id,
                                      created_at, updated_at)
                VALUES (%s, %s, %s, %s, %s, %s, %s)
                ON CONFLICT (id) DO NOTHING
                """,
                (
                    row["id"], row["content"], rust_doc_id, uid,
                    row.get("parent_id"), row["created_at"], row["updated_at"],
                ),
            )
        count += 1
    if not dry_run and count:
        dst_cur.execute("SELECT setval('comments_id_seq', (SELECT MAX(id) FROM comments))")
    return count


def migrate_document_versions(src_cur, dst_cur, default_user_id: int, dry_run: bool) -> int:
    src_cur.execute(
        "SELECT id, document_id, version_number, title, markdown_content, html_content, "
        "       author, content_hash, change_summary, created_by, created_at "
        "FROM document_versions ORDER BY id"
    )
    rows = src_cur.fetchall()
    cols = [d[0] for d in src_cur.description]
    count = 0
    for raw in rows:
        row = dict(zip(cols, raw))
        mapped = map_version_fields(row, default_user_id)
        if not dry_run:
            dst_cur.execute(
                """
                INSERT INTO document_versions (document_id, content, version_number,
                                               created_by, created_at,
                                               title, author, html_content,
                                               content_hash, change_summary)
                VALUES (%s, %s, %s, %s, %s, %s, %s, %s, %s, %s)
                ON CONFLICT (document_id, version_number) DO NOTHING
                """,
                (
                    str(mapped["document_id"]), mapped["content"], mapped["version_number"],
                    mapped["created_by"], mapped["created_at"],
                    mapped["title"], mapped["author"], mapped["html_content"],
                    mapped["content_hash"], mapped["change_summary"],
                ),
            )
        count += 1
    return count


# ─── Main ───────────────────────────────────────────────────────────────────

def run(dry_run: bool = False) -> None:
    flask_dsn = os.environ.get("FLASK_DATABASE_URL")
    rust_dsn  = os.environ.get("RUST_DATABASE_URL")
    if not flask_dsn or not rust_dsn:
        sys.exit(
            "Set FLASK_DATABASE_URL and RUST_DATABASE_URL env vars.\n"
            "Example:\n"
            "  FLASK_DATABASE_URL=postgresql://minky_app:pass@172.20.10.200:5432/minky\n"
            "  RUST_DATABASE_URL=postgresql://minky_app:pass@172.20.10.200:5432/minky_rust\n"
            "[CAS-deferred] live execution requires CAS recovery."
        )

    mode = "DRY RUN" if dry_run else "LIVE"
    print(f"=== Minky Flask→Rust migration ({mode}) ===")

    src_conn = _connect(flask_dsn)
    dst_conn = _connect(rust_dsn)

    import psycopg2.extras
    src_cur = src_conn.cursor()
    dst_cur = dst_conn.cursor()

    default_user_id = _find_default_user_id(src_cur)
    print(f"default_user_id for NULL FK rewrites: {default_user_id}")

    steps = [
        ("users",             lambda: migrate_users(src_cur, dst_cur, dry_run)),
        ("categories",        lambda: migrate_categories(src_cur, dst_cur, default_user_id, dry_run)),
        ("tags",              lambda: migrate_tags(src_cur, dst_cur, default_user_id, dry_run)),
        ("documents",         lambda: migrate_documents(src_cur, dst_cur, default_user_id, dry_run)),
        ("document_tags",     lambda: migrate_document_tags(src_cur, dst_cur, dry_run)),
        ("comments",          lambda: migrate_comments(src_cur, dst_cur, default_user_id, dry_run)),
        ("document_versions", lambda: migrate_document_versions(src_cur, dst_cur, default_user_id, dry_run)),
    ]

    try:
        for name, fn in steps:
            n = fn()
            print(f"  {name}: {n} rows {'(dry run)' if dry_run else 'migrated'}")
        if not dry_run:
            dst_conn.commit()
            print("Committed.")
        else:
            dst_conn.rollback()
            print("Dry run — rolled back.")
    except Exception as exc:
        dst_conn.rollback()
        print(f"ERROR during migration: {exc}")
        raise
    finally:
        src_conn.close()
        dst_conn.close()


if __name__ == "__main__":
    dry_run = os.environ.get("DRY_RUN", "0") not in ("0", "", "false", "False")
    run(dry_run=dry_run)
