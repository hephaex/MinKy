# Scraper Crate Migration Decision

> Evaluate migrating HTML parsing from regex to `scraper` crate for improved correctness on malformed HTML.

---

## Background

The `parse_html()` function in `minky-rust/src/pipeline/stages/parsing.rs` (lines 314–413) uses the `regex` crate to extract headings, links, and code blocks from HTML. The implementation:
- Extracts `<h1>…</h6>` via `heading_regex()` at line 25
- Extracts `<a href="…">…</a>` via `link_regex()` at line 30
- Extracts `<pre>…</pre>` and `<code>…</code>` via `pre_regex()` and `code_tag_regex()`
- Strips remaining tags and entities for plain-text body

**Known limitation:** The `regex` crate does not support backreferences, so mismatched close tags like `<h1>Foo</h3>` are silently accepted and extracted as if they were properly closed. This violates HTML5 spec parsing rules, where the parser would auto-close the `<h1>` when encountering an unexpected closing tag.

**Reference:** Sprint 26 introduced HTML heading/link extraction (commit S26-01). Comment at parsing.rs:316 notes: *"For production, consider using scraper crate"*.

---

## Dependency Footprint Comparison

| Crate | Version | Transitive deps (approx) | Binary size impact | License |
|-------|---------|--------------------------|-------------------|---------|
| `regex` | 1.x (current) | ~4 transitive (aho-corasick, regex-automata, memchr, once_cell) | baseline | MIT/Apache-2.0 |
| `scraper` | 0.20 | ~15 transitive (html5ever, markup5ever, cssparser, tendril, string_cache, phf, etc.) | +500–800 KB | MIT |

**Note:** `regex` is already in Cargo.toml; `scraper` requires a new top-level dependency. The transitive deps are largely parser and tree-building infrastructure (html5ever, tendril string interning, etc.).

---

## Correctness Comparison

| Scenario | regex path | scraper path |
|----------|-----------|--------------|
| Well-formed `<h1>Foo</h1>` | Extracts correctly | Extracts correctly |
| Mismatched close `<h1>Foo</h3>` | Extracts with wrong end (violates spec) | Handles per HTML5 spec (auto-closes h1) |
| Nested `<h1><em>Foo</em></h1>` | Extracts "Foo" (inner tag stripped) | Extracts "Foo" (text nodes collected) |
| Self-closing `<br/>` in heading | Tag stripped, emits space | Handled by tokenizer (no content) |
| Numeric entity `&#39;` | Not in decode table; preserved verbatim | Decoded by html5ever during tokenization |
| Unknown entity `&fakeentity;` | Preserved verbatim (correct) | Preserved verbatim (correct) |
| `<script>` in `<h1>` content | Script tag stripped via regex | N/A (parser separates script tokens) |

**Current behavior:** The regex approach strips inner tags for heading/link text (line 326, 370), which is correct for text extraction but fragile for malformed HTML. The scraper approach would yield identical text results but via a standards-compliant HTML5 parser.

---

## Performance Notes

- **regex (Finite Automaton):** ~1–5 µs/KB for typical HTML parsing; no catastrophic backtracking risk
- **html5ever (streaming tokenizer):** ~5–15 µs/KB depending on DOM tree depth; includes full parsing overhead
- **MinKy use case:** Individual team documents, typically < 100 KB → parsing time < 1 ms for either approach
- **Bulk ingest scenario:** For large vaults (thousands of documents × 50–100 KB each), regex has ~3x throughput advantage

**Conclusion:** Performance difference is negligible for production MinKy workloads (single-document processing); regex advantage only surfaces at bulk-ingest scale (not planned for Phase 1–2).

---

## Decision

**Recommendation: Hybrid approach — retain regex for body stripping; migrate heading/link extraction to scraper.**

**Rationale:**
1. **Correctness**: Heading/link extraction is correctness-critical; mismatched tags matter in structured data
2. **Isolation**: Plain-text body stripping does not need structural HTML correctness (goal is "extract text", not "parse structure")
3. **Cost containment**: Scraper is needed for extraction regardless; reusing it for code-block parsing adds no additional binary size
4. **Backward compatibility**: Existing test suite serves as regression suite; no API changes required

**Tracking:** Sprint 29, P1 (post-Phase 1 quality improvement)

---

## Migration Plan (Sprint 29 Preview)

1. **Add dependency:**
   ```toml
   scraper = "0.20"
   ```

2. **Add two private helper functions** in `parsing.rs`:
   ```rust
   fn scraper_extract_headings(html: &str) -> Vec<Heading> {
       use scraper::{Html, Selector};
       let doc = Html::parse_document(html);
       // Iterate h1..h6 selectors, extract text, collect positions
   }

   fn scraper_extract_links(html: &str) -> Vec<Link> {
       use scraper::{Html, Selector};
       let doc = Html::parse_document(html);
       // Iterate <a> elements, extract href + text content
   }
   ```

3. **Replace call sites** in `parse_html()` (lines 322–332, 361–375):
   ```rust
   let headings: Vec<Heading> = scraper_extract_headings(&raw.content);
   let links: Vec<Link> = scraper_extract_links(&raw.content);
   ```

4. **Retain existing regex for:**
   - Code block extraction (`pre_regex`, `code_tag_regex`, `code_language_regex`) — minimal improvement from parser
   - Body plain-text stripping (`tag_regex`, `script_regex`, `style_regex`, etc.) — performance-critical path

5. **Test strategy:**
   - Run all existing tests (lines 480–929); they serve as regression suite
   - No new test additions needed (correctness verified by existing coverage)
   - Optional: add test case for mismatched `<h1>…</h3>` to document expected behavior change

6. **Cleanup after migration:**
   - Remove `heading_regex()` and `link_regex()` OnceLock statics
   - Keep `tag_regex`, `pre_regex`, `code_tag_regex`, `code_language_regex`, `script_regex`, `style_regex`, `block_regex`, `whitespace_regex`, `title_regex`, `entity_regex` (still in use)

---

## Conclusion

Migrate heading/link extraction to `scraper` in Sprint 29; keep regex for body stripping and code-block extraction. This balances correctness improvement (malformed HTML handling) against binary size cost (~500–800 KB) and maintains performance for the non-critical paths.

---

*Prepared: 2026-05-22 | Sprint 28 S28-03*
