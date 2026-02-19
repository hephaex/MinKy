use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

// Import pure functions directly from the re-exported services
use minky::models::AuditAction;
use minky::services::{
    // audit_service pure helpers
    build_export_details,
    clamp_audit_page_params,
    is_document_action,
    is_security_sensitive,
    // comment_service pure helpers
    can_delete_comment,
    can_edit_comment,
    is_valid_comment_content,
    truncate_comment,
    // document_service pure helpers
    build_search_pattern,
    calc_offset,
    can_read_document,
    can_write_document,
    clamp_page_params,
    total_pages,
    // tag_service pure helpers
    dedup_tag_ids,
    normalize_tag_name,
    sort_tag_names,
    tags_are_duplicate,
    validate_tag_name,
};

// ---- document_service benchmarks ----

fn bench_calc_offset(c: &mut Criterion) {
    c.bench_function("calc_offset", |b| {
        b.iter(|| calc_offset(black_box(5), black_box(20)))
    });
}

fn bench_total_pages(c: &mut Criterion) {
    let mut group = c.benchmark_group("total_pages");
    for total in [0i64, 1, 100, 10_000, 1_000_000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(total), total, |b, total| {
            b.iter(|| total_pages(black_box(*total), black_box(20)))
        });
    }
    group.finish();
}

fn bench_clamp_page_params(c: &mut Criterion) {
    c.bench_function("clamp_page_params", |b| {
        b.iter(|| clamp_page_params(black_box(3), black_box(25)))
    });
}

fn bench_can_read_document(c: &mut Criterion) {
    c.bench_function("can_read_document_public", |b| {
        b.iter(|| can_read_document(black_box(1), black_box(true), black_box(99)))
    });
}

fn bench_can_write_document(c: &mut Criterion) {
    c.bench_function("can_write_document", |b| {
        b.iter(|| can_write_document(black_box(7), black_box(7)))
    });
}

fn bench_build_search_pattern(c: &mut Criterion) {
    let mut group = c.benchmark_group("build_search_pattern");
    for query in ["rust", "  Async Rust  ", "pgvector embeddings search"].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(query), query, |b, q| {
            b.iter(|| build_search_pattern(black_box(q)))
        });
    }
    group.finish();
}

// ---- tag_service benchmarks ----

fn bench_validate_tag_name(c: &mut Criterion) {
    c.bench_function("validate_tag_name_valid", |b| {
        b.iter(|| validate_tag_name(black_box("rust-async")))
    });
}

fn bench_normalize_tag_name(c: &mut Criterion) {
    c.bench_function("normalize_tag_name", |b| {
        b.iter(|| normalize_tag_name(black_box("  Rust  ")))
    });
}

fn bench_tags_are_duplicate(c: &mut Criterion) {
    c.bench_function("tags_are_duplicate", |b| {
        b.iter(|| tags_are_duplicate(black_box("Rust"), black_box("rust")))
    });
}

fn bench_sort_tag_names(c: &mut Criterion) {
    let mut group = c.benchmark_group("sort_tag_names");
    for size in [5usize, 20, 100].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let mut tags: Vec<String> = (0..size)
                    .map(|i| format!("Tag{}", size - i))
                    .collect();
                sort_tag_names(&mut tags);
                black_box(tags);
            })
        });
    }
    group.finish();
}

fn bench_dedup_tag_ids(c: &mut Criterion) {
    let mut group = c.benchmark_group("dedup_tag_ids");
    for size in [10usize, 100, 500].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                // Half the IDs are duplicates
                let ids: Vec<i32> = (0..(size as i32))
                    .chain((0..(size as i32 / 2)).map(|i| i * 2))
                    .collect();
                dedup_tag_ids(black_box(ids));
            })
        });
    }
    group.finish();
}

// ---- comment_service benchmarks ----

fn bench_can_edit_comment(c: &mut Criterion) {
    c.bench_function("can_edit_comment", |b| {
        b.iter(|| can_edit_comment(black_box(42), black_box(42)))
    });
}

fn bench_can_delete_comment(c: &mut Criterion) {
    c.bench_function("can_delete_comment_admin", |b| {
        b.iter(|| can_delete_comment(black_box(1), black_box(99), black_box(true)))
    });
}

fn bench_truncate_comment(c: &mut Criterion) {
    let long_content = "A".repeat(1000);
    c.bench_function("truncate_comment_1000chars", |b| {
        b.iter(|| truncate_comment(black_box(&long_content), black_box(200)))
    });
}

fn bench_is_valid_comment_content(c: &mut Criterion) {
    c.bench_function("is_valid_comment_content", |b| {
        b.iter(|| is_valid_comment_content(black_box("This is a valid comment.")))
    });
}

// ---- audit_service benchmarks ----

fn bench_build_export_details(c: &mut Criterion) {
    let ids: Vec<String> = (0..10).map(|i| format!("doc-id-{}", i)).collect();
    c.bench_function("build_export_details_10docs", |b| {
        b.iter(|| build_export_details(black_box(&ids), black_box("json")))
    });
}

fn bench_is_security_sensitive(c: &mut Criterion) {
    c.bench_function("is_security_sensitive", |b| {
        b.iter(|| is_security_sensitive(black_box(&AuditAction::LoginFailed)))
    });
}

fn bench_is_document_action(c: &mut Criterion) {
    c.bench_function("is_document_action", |b| {
        b.iter(|| is_document_action(black_box(&AuditAction::Create)))
    });
}

fn bench_clamp_audit_page_params(c: &mut Criterion) {
    c.bench_function("clamp_audit_page_params", |b| {
        b.iter(|| clamp_audit_page_params(black_box(50), black_box(100)))
    });
}

criterion_group!(
    benches,
    bench_calc_offset,
    bench_total_pages,
    bench_clamp_page_params,
    bench_can_read_document,
    bench_can_write_document,
    bench_build_search_pattern,
    bench_validate_tag_name,
    bench_normalize_tag_name,
    bench_tags_are_duplicate,
    bench_sort_tag_names,
    bench_dedup_tag_ids,
    bench_can_edit_comment,
    bench_can_delete_comment,
    bench_truncate_comment,
    bench_is_valid_comment_content,
    bench_build_export_details,
    bench_is_security_sensitive,
    bench_is_document_action,
    bench_clamp_audit_page_params,
);
criterion_main!(benches);
