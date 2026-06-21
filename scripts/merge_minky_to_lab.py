#!/usr/bin/env python3
"""
merge_minky_to_lab.py — Merge source `minky` (Flask/CAS CT250) INTO target `minky_lab` (Rust/GWA CT306).

Additive merge: target already has 900 docs we MUST preserve.
Default mode: --dry-run (ZERO writes). Pass --live to execute (separate step, requires explicit GO).

Data path:
  - Source exports:  /tmp/minky_export/src_docs_meta.psv  (4,780 rows, pipe-sep)
                     /tmp/minky_export/src_tags.json       (753 rows)
                     /tmp/minky_export/src_doc_tags.json   (8,392 rows)
  - Target exports:  /tmp/minky_export/tgt_tags.json       (169 rows)
                     /tmp/minky_export/tgt_doc_ids.json    (900 UUIDs)

Transform rules (FINAL — do not change without new decision):
  documents:
    id            → uuid5(NAMESPACE, f"minky:document:{flask_id}")
    content       ← markdown_content (may be NULL → flag)
    user_id       → 2  (fixed: reuse existing lab user 2)
    category_id   → NULL
    sensitivity   → 'internal'
    collection_id → NULL
    view_count    → 0
    preserve: title, author, html_content, document_metadata, is_public,
              is_published, published_at, created_at, updated_at

  tags (dedup by name, case-insensitive):
    if target already has tag with same name → reuse target id (no insert)
    else → insert with id = current_max_target_tag_id + N (incrementing)
    carry: name, slug, description, color

  document_tags (8,392 rows):
    remap document_id → UUID per documents rule
    remap tag_id      → resolved target tag id per dedup map
    skip if document not in migrated set (shouldn't happen) or tag remap failed

Usage:
    python3 scripts/merge_minky_to_lab.py          # dry-run (default)
    python3 scripts/merge_minky_to_lab.py --dry-run
    python3 scripts/merge_minky_to_lab.py --live   # NOT RUN in this task — build only

For live mode, set env:
    TARGET_DSN=postgresql://minky_lab:PASS@192.168.29.22:PORT/minky_lab
"""

from __future__ import annotations

import argparse
import csv
import json
import os
import sys
import uuid
from collections import defaultdict
from pathlib import Path
from typing import Any

# ─── Constants ──────────────────────────────────────────────────────────────

MIGRATION_NAMESPACE = uuid.UUID("1d6b1000-6b40-5000-a000-000000000001")

EXPORT_DIR = Path("/tmp/minky_export")

SRC_DOCS_META_JSON = EXPORT_DIR / "src_docs_meta.json"
SRC_TAGS_JSON      = EXPORT_DIR / "src_tags.json"
SRC_DOC_TAGS_JSON  = EXPORT_DIR / "src_doc_tags.json"
TGT_TAGS_JSON      = EXPORT_DIR / "tgt_tags.json"
TGT_DOC_IDS_JSON   = EXPORT_DIR / "tgt_doc_ids.json"

# Fixed transform values
FIXED_USER_ID   = 2
FIXED_CAT_ID    = None
FIXED_SENSITIVITY = "internal"
FIXED_VIEW_COUNT  = 0
FIXED_COLLECTION_ID = None

# ─── Core transform functions ────────────────────────────────────────────────

def make_document_uuid(flask_id: int) -> str:
    """Deterministic UUIDv5 from source integer id. Same as migrate_flask_to_rust.py."""
    return str(uuid.uuid5(MIGRATION_NAMESPACE, f"minky:document:{flask_id}"))


def parse_bool_col(v: str) -> bool:
    """Parse psql boolean: 't'/'f' or '1'/'0'."""
    return v.strip().lower() in ("t", "true", "1")


def parse_docs_json(path: Path) -> list[dict[str, Any]]:
    """Parse docs_meta.json — list of CSV rows (list-of-lists) produced by
    Python csv.reader inside CT250.

    Columns (13):
      0  id
      1  title
      2  user_id
      3  category_id
      4  is_public        (t/f)
      5  is_published     (t/f)
      6  published_at
      7  created_at
      8  updated_at
      9  author
      10 content_null     (0/1)
      11 html_null        (0/1)
      12 meta_null        (0/1)
    """
    raw = json.loads(path.read_text(encoding="utf-8"))
    docs = []
    for idx, parts in enumerate(raw):
        if len(parts) < 13:
            print(f"  WARN: row {idx} has only {len(parts)} fields, skipping")
            continue
        docs.append({
            "id":           int(parts[0]),
            "title":        parts[1],
            "user_id":      int(parts[2]) if parts[2] else None,
            "category_id":  int(parts[3]) if parts[3] else None,
            "is_public":    parse_bool_col(parts[4]),
            "is_published": parse_bool_col(parts[5]),
            "published_at": parts[6] if parts[6] else None,
            "created_at":   parts[7],
            "updated_at":   parts[8],
            "author":       parts[9] if parts[9] else None,
            "content_null": parts[10] == "1",
            "html_null":    parts[11] == "1",
            "meta_null":    parts[12] == "1",
        })
    return docs


# ─── Dry-run compute engine ──────────────────────────────────────────────────

def compute_dry_run() -> None:
    print("=" * 70)
    print("MERGE minky → minky_lab  [DRY-RUN — ZERO WRITES]")
    print("=" * 70)

    # ── Load all exports ─────────────────────────────────────────────────────
    print("\n[1/5] Loading export files …")
    src_docs    = parse_docs_json(SRC_DOCS_META_JSON)
    src_tags    = json.loads(SRC_TAGS_JSON.read_text())
    src_dt      = json.loads(SRC_DOC_TAGS_JSON.read_text())
    tgt_tags    = json.loads(TGT_TAGS_JSON.read_text())
    tgt_doc_ids = json.loads(TGT_DOC_IDS_JSON.read_text())

    print(f"  src documents:    {len(src_docs):,}")
    print(f"  src tags:         {len(src_tags):,}")
    print(f"  src document_tags:{len(src_dt):,}")
    print(f"  tgt tags:         {len(tgt_tags):,}")
    print(f"  tgt doc UUIDs:    {len(tgt_doc_ids):,}")

    # ── DOCUMENTS ────────────────────────────────────────────────────────────
    print("\n[2/5] Analysing DOCUMENTS …")

    # Build set of existing target UUIDs (lower-cased for safe comparison)
    tgt_uuid_set = {r["id"].lower() for r in tgt_doc_ids}

    doc_uuid_map: dict[int, str] = {}   # flask_id → target UUID
    docs_to_insert: list[dict] = []
    docs_collision: list[tuple[int, str]] = []
    anomalies: dict[str, list] = defaultdict(list)

    for d in src_docs:
        fid  = d["id"]
        new_uuid = make_document_uuid(fid)
        doc_uuid_map[fid] = new_uuid

        if new_uuid.lower() in tgt_uuid_set:
            docs_collision.append((fid, new_uuid))

        if d["content_null"]:
            anomalies["null_content"].append(fid)
        if d["title"] == "":
            anomalies["empty_title"].append(fid)

        docs_to_insert.append({
            "flask_id":      fid,
            "id":            new_uuid,
            "title":         d["title"],
            "user_id":       FIXED_USER_ID,
            "category_id":   FIXED_CAT_ID,
            "sensitivity":   FIXED_SENSITIVITY,
            "collection_id": FIXED_COLLECTION_ID,
            "view_count":    FIXED_VIEW_COUNT,
            "is_public":     d["is_public"],
            "is_published":  d["is_published"],
            "published_at":  d["published_at"],
            "created_at":    d["created_at"],
            "updated_at":    d["updated_at"],
            "author":        d["author"],
            # content, html_content, document_metadata come from live DB in live mode
            "has_content":   not d["content_null"],
            "has_html":      not d["html_null"],
            "has_meta":      not d["meta_null"],
        })

    print(f"  docs to INSERT:         {len(docs_to_insert):,}")
    print(f"  UUID collisions w/ tgt: {len(docs_collision)}")
    if docs_collision:
        print(f"  COLLISION DETAIL: {docs_collision[:5]}")
    print(f"  anomalies — null content:  {len(anomalies['null_content'])}")
    print(f"  anomalies — empty title:   {len(anomalies['empty_title'])}")
    if anomalies["null_content"]:
        print(f"    null content flask_ids (first 10): {anomalies['null_content'][:10]}")

    # ── TAGS ─────────────────────────────────────────────────────────────────
    print("\n[3/5] Analysing TAGS (dedup by name, case-insensitive) …")

    # Build target name→id map (lower-cased)
    tgt_name_to_id: dict[str, int] = {t["name"].lower(): t["id"] for t in tgt_tags}
    tgt_max_id = max(t["id"] for t in tgt_tags)

    # Detect duplicate names within source tags
    src_name_counts: dict[str, list[int]] = defaultdict(list)
    for t in src_tags:
        src_name_counts[t["name"].lower()].append(t["id"])
    src_dup_names = {n: ids for n, ids in src_name_counts.items() if len(ids) > 1}

    # Build tag remap: src tag id → target tag id
    src_tag_remap: dict[int, int] = {}
    tags_reused:   list[dict] = []      # matched by name in target
    tags_new:      list[dict] = []      # need to insert
    next_id = tgt_max_id + 1

    for t in sorted(src_tags, key=lambda x: x["id"]):
        name_lower = t["name"].lower()
        if name_lower in tgt_name_to_id:
            # Reuse existing target tag
            tid = tgt_name_to_id[name_lower]
            src_tag_remap[t["id"]] = tid
            tags_reused.append({"src_id": t["id"], "tgt_id": tid, "name": t["name"]})
        else:
            # New tag — assign next sequential id
            src_tag_remap[t["id"]] = next_id
            tags_new.append({
                "src_id":      t["id"],
                "tgt_id":      next_id,
                "name":        t["name"],
                "slug":        t.get("slug"),
                "description": t.get("description"),
                "color":       t.get("color", "#007bff"),
            })
            tgt_name_to_id[name_lower] = next_id  # prevent same name inserting twice
            next_id += 1

    new_id_range = (tags_new[0]["tgt_id"], tags_new[-1]["tgt_id"]) if tags_new else (None, None)

    print(f"  src tags total:    {len(src_tags):,}")
    print(f"  reused by name:    {len(tags_reused):,}")
    print(f"  new to insert:     {len(tags_new):,}")
    print(f"  new id range:      {new_id_range[0]} – {new_id_range[1]}")
    print(f"  dup names in src:  {len(src_dup_names)}")
    if src_dup_names:
        for n, ids in list(src_dup_names.items())[:5]:
            print(f"    '{n}': src ids {ids}")

    # ── DOCUMENT_TAGS ────────────────────────────────────────────────────────
    print("\n[4/5] Analysing DOCUMENT_TAGS …")

    src_doc_id_set = set(doc_uuid_map.keys())  # all flask doc ids

    dt_to_insert: list[dict] = []
    dt_skip_no_doc:  list[dict] = []
    dt_skip_no_tag:  list[dict] = []
    seen_pairs: set[tuple[str, int]] = set()
    dt_skip_dup: int = 0

    for row in src_dt:
        fdoc_id  = row["document_id"]
        src_t_id = row["tag_id"]

        if fdoc_id not in src_doc_id_set:
            dt_skip_no_doc.append(row)
            continue

        if src_t_id not in src_tag_remap:
            dt_skip_no_tag.append(row)
            continue

        tgt_doc_uuid = doc_uuid_map[fdoc_id]
        tgt_t_id     = src_tag_remap[src_t_id]
        pair = (tgt_doc_uuid, tgt_t_id)
        if pair in seen_pairs:
            dt_skip_dup += 1
            continue
        seen_pairs.add(pair)

        dt_to_insert.append({
            "document_id": tgt_doc_uuid,
            "tag_id":      tgt_t_id,
        })

    print(f"  src document_tags:       {len(src_dt):,}")
    print(f"  to insert after remap:   {len(dt_to_insert):,}")
    print(f"  skipped — no doc found:  {len(dt_skip_no_doc)}")
    print(f"  skipped — no tag found:  {len(dt_skip_no_tag)}")
    print(f"  skipped — duplicate pair:{dt_skip_dup}")

    # ── SUMMARY ──────────────────────────────────────────────────────────────
    print("\n" + "=" * 70)
    print("DRY-RUN SUMMARY")
    print("=" * 70)
    print(f"DOCUMENTS")
    print(f"  Source docs:              {len(src_docs):,}")
    print(f"  To INSERT into minky_lab: {len(docs_to_insert):,}")
    print(f"  UUID collision w/ 900 tgt:{len(docs_collision)}  ← must be 0")
    print(f"  Target preserved (900):   UNTOUCHED")
    print()
    print(f"TAGS")
    print(f"  Source tags:              {len(src_tags):,}")
    print(f"  Reused (name match):      {len(tags_reused):,}")
    print(f"  New to INSERT:            {len(tags_new):,}")
    print(f"  New id range:             {new_id_range[0]} – {new_id_range[1]}")
    print()
    print(f"DOCUMENT_TAGS")
    print(f"  Source rows:              {len(src_dt):,}")
    print(f"  To INSERT after remap:    {len(dt_to_insert):,}")
    print(f"  Skipped (no doc match):   {len(dt_skip_no_doc)}")
    print(f"  Skipped (no tag match):   {len(dt_skip_no_tag)}")
    print(f"  Skipped (dup pair):       {dt_skip_dup}")
    print()
    print(f"ANOMALIES")
    print(f"  Docs with NULL content:   {len(anomalies['null_content'])}")
    print(f"  Docs with empty title:    {len(anomalies['empty_title'])}")
    print(f"  Dup tag names in source:  {len(src_dup_names)}")
    print()
    print(f"WRITES PERFORMED: 0  ← DRY RUN CONFIRMED")
    print("=" * 70)

    return {
        "docs_to_insert": len(docs_to_insert),
        "uuid_collisions": len(docs_collision),
        "tags_reused": len(tags_reused),
        "tags_new": len(tags_new),
        "new_tag_id_range": new_id_range,
        "dt_to_insert": len(dt_to_insert),
        "dt_skip_no_doc": len(dt_skip_no_doc),
        "dt_skip_no_tag": len(dt_skip_no_tag),
        "dt_skip_dup": dt_skip_dup,
        "null_content_docs": anomalies["null_content"],
        "dup_src_tag_names": src_dup_names,
    }


# ─── Live mode (build but DO NOT RUN in this task) ───────────────────────────

def run_live() -> None:
    """Live merge — wraps ALL inserts in a single transaction. Idempotent.

    DO NOT CALL until explicit user GO after dry-run review.

    Requires:
        TARGET_DSN env var — postgresql://minky_lab:PASS@host:PORT/minky_lab

    For source content we re-query source DB at insert time (not stored in exports).
    Requires:
        SOURCE_DSN env var — postgresql://minky_app:PASS@host:PORT/minky
    """
    try:
        import psycopg2
        import psycopg2.extras
    except ImportError:
        sys.exit("Live mode requires psycopg2: pip install psycopg2-binary")

    src_dsn = os.environ.get("SOURCE_DSN")
    tgt_dsn = os.environ.get("TARGET_DSN")
    if not src_dsn or not tgt_dsn:
        sys.exit("Set SOURCE_DSN and TARGET_DSN environment variables for live mode.")

    print("LIVE MODE — connecting to both DBs …")
    src_conn = psycopg2.connect(src_dsn, client_encoding="utf8")
    tgt_conn = psycopg2.connect(tgt_dsn, client_encoding="utf8")
    src_conn.autocommit = True   # read-only, no transaction needed
    tgt_conn.autocommit = False  # explicit transaction

    src_cur = src_conn.cursor(cursor_factory=psycopg2.extras.RealDictCursor)
    tgt_cur = tgt_conn.cursor()

    # Re-fetch source data from live DB for content fields
    src_cur.execute("""
        SELECT id, title, markdown_content AS content, author, html_content,
               document_metadata, is_public, is_published, published_at,
               created_at, updated_at
        FROM documents ORDER BY id
    """)
    src_docs_live = src_cur.fetchall()

    src_tags    = json.loads(SRC_TAGS_JSON.read_text())
    src_dt      = json.loads(SRC_DOC_TAGS_JSON.read_text())
    tgt_tags    = json.loads(TGT_TAGS_JSON.read_text())

    tgt_name_to_id = {t["name"].lower(): t["id"] for t in tgt_tags}
    tgt_max_id = max(t["id"] for t in tgt_tags)
    next_id = tgt_max_id + 1

    src_tag_remap: dict[int, int] = {}
    tags_new: list[dict] = []
    for t in sorted(src_tags, key=lambda x: x["id"]):
        name_lower = t["name"].lower()
        if name_lower in tgt_name_to_id:
            src_tag_remap[t["id"]] = tgt_name_to_id[name_lower]
        else:
            src_tag_remap[t["id"]] = next_id
            tags_new.append({"src_id": t["id"], "tgt_id": next_id,
                             "name": t["name"], "slug": t.get("slug"),
                             "description": t.get("description"),
                             "color": t.get("color", "#007bff")})
            tgt_name_to_id[name_lower] = next_id
            next_id += 1

    doc_uuid_map: dict[int, str] = {}

    try:
        # Disable the tsvector trigger for bulk import (re-enabled after commit)
        # Large documents (>1MB content) overflow tsvector's 1048575-byte limit.
        # The search vector will be rebuilt on next UPDATE or can be re-triggered manually.
        tgt_cur.execute("ALTER TABLE documents DISABLE TRIGGER trig_documents_search_vector")

        # 1. Insert new tags
        for t in tags_new:
            tgt_cur.execute("""
                INSERT INTO tags (id, name, slug, description, color)
                VALUES (%s, %s, %s, %s, %s)
                ON CONFLICT (id) DO NOTHING
            """, (t["tgt_id"], t["name"], t["slug"], t["description"], t["color"]))

        # 2. Insert documents
        for d in src_docs_live:
            fid = d["id"]
            new_uuid = make_document_uuid(fid)
            doc_uuid_map[fid] = new_uuid
            meta = json.dumps(d["document_metadata"]) if d.get("document_metadata") else None
            tgt_cur.execute("""
                INSERT INTO documents (
                    id, title, content, author, html_content, document_metadata,
                    user_id, category_id, sensitivity, collection_id,
                    is_public, is_published, published_at, view_count,
                    created_at, updated_at
                ) VALUES (
                    %s, %s, %s, %s, %s, %s::jsonb,
                    %s, %s, %s, %s,
                    %s, %s, %s, %s,
                    %s, %s
                )
                ON CONFLICT (id) DO NOTHING
            """, (
                new_uuid, d["title"], d["content"], d.get("author"), d.get("html_content"), meta,
                FIXED_USER_ID, FIXED_CAT_ID, FIXED_SENSITIVITY, FIXED_COLLECTION_ID,
                d.get("is_public", True), d.get("is_published", False),
                d.get("published_at"), FIXED_VIEW_COUNT,
                d["created_at"], d["updated_at"],
            ))

        # 3. Insert document_tags
        src_doc_id_set = set(doc_uuid_map.keys())
        seen_pairs: set[tuple[str, int]] = set()
        for row in src_dt:
            fdoc_id  = row["document_id"]
            src_t_id = row["tag_id"]
            if fdoc_id not in src_doc_id_set:
                continue
            if src_t_id not in src_tag_remap:
                continue
            tgt_doc_uuid = doc_uuid_map[fdoc_id]
            tgt_t_id     = src_tag_remap[src_t_id]
            pair = (tgt_doc_uuid, tgt_t_id)
            if pair in seen_pairs:
                continue
            seen_pairs.add(pair)
            tgt_cur.execute("""
                INSERT INTO document_tags (document_id, tag_id)
                VALUES (%s, %s)
                ON CONFLICT (document_id, tag_id) DO NOTHING
            """, (tgt_doc_uuid, tgt_t_id))

        # Re-enable trigger before commit so it's back in place for future writes
        tgt_cur.execute("ALTER TABLE documents ENABLE TRIGGER trig_documents_search_vector")
        tgt_conn.commit()
        print("LIVE merge committed successfully.")

    except Exception as exc:
        tgt_conn.rollback()
        print(f"ERROR — rolled back: {exc}")
        raise
    finally:
        src_conn.close()
        tgt_conn.close()


# ─── Entry point ─────────────────────────────────────────────────────────────

def main() -> None:
    parser = argparse.ArgumentParser(
        description="Merge minky (Flask) into minky_lab (Rust) — additive, preserves 900 existing docs."
    )
    parser.add_argument(
        "--live",
        action="store_true",
        help="Execute live inserts (requires SOURCE_DSN + TARGET_DSN env vars). "
             "DO NOT pass this flag until dry-run has been reviewed and user has given GO.",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        default=True,
        help="Compute full transform, print report, zero writes (default).",
    )
    args = parser.parse_args()

    if args.live:
        run_live()
    else:
        compute_dry_run()


if __name__ == "__main__":
    main()
