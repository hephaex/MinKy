#!/usr/bin/env python3
"""
Migration verification: row-count / content-hash sample / FK orphan checks.

[CAS-deferred] Requires live DB access. Run after migrate_flask_to_rust.py.

Usage:
    FLASK_DATABASE_URL=... RUST_DATABASE_URL=... python scripts/verify_migration.py
"""

from __future__ import annotations

import hashlib
import os
import sys
from typing import Any

sys.path.insert(0, os.path.dirname(__file__))
from migrate_flask_to_rust import _connect, make_document_uuid


SAMPLE_SIZE = 50  # documents to hash-compare


def check_row_counts(src_cur, dst_cur) -> list[str]:
    """Verify every migrated table has same row count in source and target."""
    tables = ["users", "categories", "tags", "documents", "document_tags", "comments", "document_versions"]
    failures = []
    print("\n── Row count check ──")
    for table in tables:
        src_cur.execute(f"SELECT COUNT(*) FROM {table}")
        src_n = src_cur.fetchone()[0]
        dst_cur.execute(f"SELECT COUNT(*) FROM {table}")
        dst_n = dst_cur.fetchone()[0]
        status = "OK" if src_n == dst_n else "FAIL"
        print(f"  {table}: source={src_n} target={dst_n} [{status}]")
        if src_n != dst_n:
            failures.append(f"{table}: source={src_n} != target={dst_n}")
    return failures


def check_mapping_completeness(src_cur, dst_cur) -> list[str]:
    """Every Flask document id must appear in flask_document_id_mapping."""
    src_cur.execute("SELECT COUNT(*) FROM documents")
    src_n = src_cur.fetchone()[0]
    dst_cur.execute("SELECT COUNT(*) FROM flask_document_id_mapping")
    map_n = dst_cur.fetchone()[0]
    failures = []
    status = "OK" if src_n == map_n else "FAIL"
    print(f"\n── ID mapping completeness: source={src_n} mapping={map_n} [{status}]")
    if src_n != map_n:
        failures.append(f"flask_document_id_mapping: {map_n} entries vs {src_n} source documents")
    return failures


def check_content_hash_sample(src_cur, dst_cur) -> list[str]:
    """Sample SAMPLE_SIZE documents and compare content hashes."""
    src_cur.execute(
        "SELECT id, markdown_content FROM documents ORDER BY id LIMIT %s",
        (SAMPLE_SIZE,),
    )
    src_rows = src_cur.fetchall()
    failures = []
    mismatches = 0
    print(f"\n── Content hash sample (n={len(src_rows)}) ──")
    for flask_id, markdown_content in src_rows:
        rust_id = str(make_document_uuid(flask_id))
        dst_cur.execute("SELECT content FROM documents WHERE id = %s", (rust_id,))
        dst_row = dst_cur.fetchone()
        if dst_row is None:
            failures.append(f"document flask_id={flask_id} rust_id={rust_id}: NOT FOUND in target")
            mismatches += 1
            continue
        src_hash = hashlib.sha256((markdown_content or "").encode("utf-8")).hexdigest()
        dst_hash = hashlib.sha256((dst_row[0] or "").encode("utf-8")).hexdigest()
        if src_hash != dst_hash:
            failures.append(f"document flask_id={flask_id}: content hash mismatch")
            mismatches += 1
    print(f"  Checked {len(src_rows)}, mismatches: {mismatches}")
    return failures


def check_fk_orphans(dst_cur) -> list[str]:
    """All FK references in target must resolve (no orphans)."""
    checks = [
        # (description, query that should return 0 rows)
        (
            "document_tags → documents orphans",
            """
            SELECT COUNT(*) FROM document_tags dt
            WHERE NOT EXISTS (SELECT 1 FROM documents d WHERE d.id = dt.document_id)
            """,
        ),
        (
            "document_tags → tags orphans",
            """
            SELECT COUNT(*) FROM document_tags dt
            WHERE NOT EXISTS (SELECT 1 FROM tags t WHERE t.id = dt.tag_id)
            """,
        ),
        (
            "comments → documents orphans",
            """
            SELECT COUNT(*) FROM comments c
            WHERE NOT EXISTS (SELECT 1 FROM documents d WHERE d.id = c.document_id)
            """,
        ),
        (
            "comments → users orphans",
            """
            SELECT COUNT(*) FROM comments c
            WHERE NOT EXISTS (SELECT 1 FROM users u WHERE u.id = c.user_id)
            """,
        ),
        (
            "document_versions → documents orphans",
            """
            SELECT COUNT(*) FROM document_versions dv
            WHERE NOT EXISTS (SELECT 1 FROM documents d WHERE d.id = dv.document_id)
            """,
        ),
        (
            "documents → users orphans",
            """
            SELECT COUNT(*) FROM documents d
            WHERE NOT EXISTS (SELECT 1 FROM users u WHERE u.id = d.user_id)
            """,
        ),
        (
            "documents → categories orphans",
            """
            SELECT COUNT(*) FROM documents d
            WHERE d.category_id IS NOT NULL
              AND NOT EXISTS (SELECT 1 FROM categories c WHERE c.id = d.category_id)
            """,
        ),
    ]
    failures = []
    print("\n── FK orphan check ──")
    for desc, query in checks:
        dst_cur.execute(query)
        orphan_count = dst_cur.fetchone()[0]
        status = "OK" if orphan_count == 0 else "FAIL"
        print(f"  {desc}: {orphan_count} orphans [{status}]")
        if orphan_count > 0:
            failures.append(f"{desc}: {orphan_count} orphans")
    return failures


def run() -> None:
    flask_dsn = os.environ.get("FLASK_DATABASE_URL")
    rust_dsn  = os.environ.get("RUST_DATABASE_URL")
    if not flask_dsn or not rust_dsn:
        sys.exit("Set FLASK_DATABASE_URL and RUST_DATABASE_URL. [CAS-deferred]")

    src_conn = _connect(flask_dsn)
    dst_conn = _connect(rust_dsn)
    src_cur = src_conn.cursor()
    dst_cur = dst_conn.cursor()

    print("=== Migration Verification ===")

    all_failures: list[str] = []
    all_failures += check_row_counts(src_cur, dst_cur)
    all_failures += check_mapping_completeness(src_cur, dst_cur)
    all_failures += check_content_hash_sample(src_cur, dst_cur)
    all_failures += check_fk_orphans(dst_cur)

    src_conn.close()
    dst_conn.close()

    print("\n── Summary ──")
    if not all_failures:
        print("  ALL CHECKS PASSED ✓")
        sys.exit(0)
    else:
        print(f"  {len(all_failures)} FAILURE(S):")
        for f in all_failures:
            print(f"    ✗ {f}")
        sys.exit(1)


if __name__ == "__main__":
    run()
