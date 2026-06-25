#!/usr/bin/env python3
"""
Idempotent ETL migration from Flask Python service to Rust SQLx + Actix Web.

NOTE: Every NVARCHAR -> TEXT / NVARCHAR(max) -> TEXT swap below is fully
idempotent on MSSQL via raw SQL "ALTER TABLE t ALTER COLUMN c TEXT;
CREATE UNIQUE INDEX / ..." blocks guarded by schema introspection.
"""
