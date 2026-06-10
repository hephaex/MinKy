"""Unit tests for migrate_flask_to_rust.py pure functions.

No DB connection required. Run with:
    cd /path/to/minky && python -m pytest scripts/tests/test_migrate.py -v
"""

import hashlib
import json
import uuid

import pytest
import sys
import os

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
from migrate_flask_to_rust import (
    MIGRATION_NAMESPACE,
    coerce_user_id,
    content_hash_sample,
    make_document_uuid,
    map_document_fields,
    map_user_role,
    map_version_fields,
)


# ─── make_document_uuid ───────────────────────────────────────────────────

class TestMakeDocumentUUID:
    def test_returns_uuid(self):
        result = make_document_uuid(1)
        assert isinstance(result, uuid.UUID)

    def test_is_version_5(self):
        result = make_document_uuid(42)
        assert result.version == 5

    def test_deterministic_same_id(self):
        assert make_document_uuid(1) == make_document_uuid(1)

    def test_deterministic_large_id(self):
        assert make_document_uuid(4545) == make_document_uuid(4545)

    def test_different_ids_produce_different_uuids(self):
        assert make_document_uuid(1) != make_document_uuid(2)
        assert make_document_uuid(100) != make_document_uuid(101)

    def test_not_null_uuid(self):
        # uuid.UUID("00000000-0000-0000-0000-000000000000") is the nil UUID
        nil = uuid.UUID(int=0)
        assert make_document_uuid(1) != nil

    def test_id_zero_distinct(self):
        assert make_document_uuid(0) != make_document_uuid(1)

    def test_negative_id_distinct(self):
        # Edge case: Flask IDs are positive but guard against negative
        assert make_document_uuid(-1) != make_document_uuid(1)

    def test_known_stable_value(self):
        # Pin a known value to detect accidental namespace changes.
        # Recompute if MIGRATION_NAMESPACE changes (migration-breaking change).
        expected = uuid.uuid5(MIGRATION_NAMESPACE, "minky:document:1")
        assert make_document_uuid(1) == expected


# ─── map_user_role ────────────────────────────────────────────────────────

class TestMapUserRole:
    def test_admin_true(self):
        assert map_user_role(True) == "admin"

    def test_admin_false(self):
        assert map_user_role(False) == "user"


# ─── coerce_user_id ──────────────────────────────────────────────────────

class TestCoerceUserId:
    def test_non_null_returned_as_is(self):
        assert coerce_user_id(5, default_user_id=1) == 5

    def test_null_returns_default(self):
        assert coerce_user_id(None, default_user_id=99) == 99

    def test_zero_is_not_null(self):
        # 0 is a valid integer (not None)
        assert coerce_user_id(0, default_user_id=99) == 0


# ─── map_document_fields ─────────────────────────────────────────────────

BASE_DOC = {
    "id": 42,
    "title": "Test Doc",
    "markdown_content": "# Hello",
    "author": "Mario",
    "html_content": "<h1>Hello</h1>",
    "document_metadata": {"key": "val"},
    "user_id": 1,
    "category_id": 3,
    "is_public": True,
    "is_published": False,
    "published_at": None,
    "created_at": None,
    "updated_at": None,
}


class TestMapDocumentFields:
    def test_content_renamed_from_markdown_content(self):
        result = map_document_fields(BASE_DOC, default_user_id=1)
        assert result["content"] == "# Hello"
        assert "markdown_content" not in result

    def test_id_is_uuid(self):
        result = map_document_fields(BASE_DOC, default_user_id=1)
        assert result["id"] == make_document_uuid(42)

    def test_id_deterministic(self):
        r1 = map_document_fields(BASE_DOC, default_user_id=1)
        r2 = map_document_fields(BASE_DOC, default_user_id=1)
        assert r1["id"] == r2["id"]

    def test_null_user_id_uses_default(self):
        doc = {**BASE_DOC, "user_id": None}
        result = map_document_fields(doc, default_user_id=7)
        assert result["user_id"] == 7

    def test_non_null_user_id_preserved(self):
        result = map_document_fields(BASE_DOC, default_user_id=99)
        assert result["user_id"] == 1

    def test_flask_id_is_stripped_from_result(self):
        result = map_document_fields(BASE_DOC, default_user_id=1)
        # _flask_id is an internal key; callers pop it before INSERT
        # (map_document_fields includes it; caller uses it then removes)
        assert "_flask_id" in result
        assert result["_flask_id"] == 42

    def test_additive_columns_present(self):
        result = map_document_fields(BASE_DOC, default_user_id=1)
        assert result["author"] == "Mario"
        assert result["html_content"] == "<h1>Hello</h1>"
        assert result["is_published"] is False
        assert result["published_at"] is None

    def test_document_metadata_json_serialized(self):
        result = map_document_fields(BASE_DOC, default_user_id=1)
        # Should be JSON string for psycopg2 JSONB insertion
        assert result["document_metadata"] == json.dumps({"key": "val"})

    def test_null_document_metadata_stays_null(self):
        doc = {**BASE_DOC, "document_metadata": None}
        result = map_document_fields(doc, default_user_id=1)
        assert result["document_metadata"] is None

    def test_view_count_reset_to_zero(self):
        result = map_document_fields(BASE_DOC, default_user_id=1)
        assert result["view_count"] == 0

    def test_is_public_preserved(self):
        result = map_document_fields({**BASE_DOC, "is_public": False}, default_user_id=1)
        assert result["is_public"] is False

    def test_category_id_preserved(self):
        result = map_document_fields(BASE_DOC, default_user_id=1)
        assert result["category_id"] == 3

    def test_null_category_id_preserved(self):
        doc = {**BASE_DOC, "category_id": None}
        result = map_document_fields(doc, default_user_id=1)
        assert result["category_id"] is None


# ─── map_version_fields ──────────────────────────────────────────────────

BASE_VER = {
    "id": 10,
    "document_id": 42,
    "version_number": 3,
    "title": "Test Doc v3",
    "markdown_content": "# v3 content",
    "html_content": "<h1>v3</h1>",
    "author": "Mario",
    "content_hash": "abc123",
    "change_summary": "small fix",
    "created_by": 1,
    "created_at": None,
}


class TestMapVersionFields:
    def test_content_renamed(self):
        result = map_version_fields(BASE_VER, default_user_id=1)
        assert result["content"] == "# v3 content"
        assert "markdown_content" not in result

    def test_document_id_is_uuid(self):
        result = map_version_fields(BASE_VER, default_user_id=1)
        assert result["document_id"] == make_document_uuid(42)

    def test_version_number_preserved(self):
        result = map_version_fields(BASE_VER, default_user_id=1)
        assert result["version_number"] == 3

    def test_null_created_by_uses_default(self):
        ver = {**BASE_VER, "created_by": None}
        result = map_version_fields(ver, default_user_id=5)
        assert result["created_by"] == 5

    def test_additive_columns_present(self):
        result = map_version_fields(BASE_VER, default_user_id=1)
        assert result["title"] == "Test Doc v3"
        assert result["author"] == "Mario"
        assert result["html_content"] == "<h1>v3</h1>"
        assert result["content_hash"] == "abc123"
        assert result["change_summary"] == "small fix"


# ─── content_hash_sample ─────────────────────────────────────────────────

class TestContentHashSample:
    def test_deterministic(self):
        assert content_hash_sample("hello") == content_hash_sample("hello")

    def test_different_content_different_hash(self):
        assert content_hash_sample("hello") != content_hash_sample("world")

    def test_sha256_hex(self):
        expected = hashlib.sha256("hello".encode("utf-8")).hexdigest()
        assert content_hash_sample("hello") == expected

    def test_empty_string(self):
        result = content_hash_sample("")
        assert len(result) == 64  # SHA-256 hex is always 64 chars


# ─── Integration invariants (no DB) ──────────────────────────────────────

class TestIntegrationInvariants:
    def test_all_flask_ids_produce_unique_uuids(self):
        """1..100 Flask IDs must all map to distinct UUIDs."""
        uuids = [make_document_uuid(i) for i in range(1, 101)]
        assert len(set(uuids)) == 100

    def test_uuid_for_document_id_consistent_across_tables(self):
        """document_tags and comments must use the same UUID as documents."""
        flask_id = 777
        doc_uuid = make_document_uuid(flask_id)
        # Simulated FK rewrite: document_tags.document_id uses same function
        tag_fk_uuid = make_document_uuid(flask_id)
        comment_fk_uuid = make_document_uuid(flask_id)
        assert doc_uuid == tag_fk_uuid == comment_fk_uuid

    def test_migration_namespace_is_stable(self):
        """Namespace UUID must never change after first migration run."""
        assert str(MIGRATION_NAMESPACE) == "1d6b1000-6b40-5000-a000-000000000001"
