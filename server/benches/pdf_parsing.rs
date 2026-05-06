use std::{fs, hint::black_box, time::Duration};

use criterion::{Criterion, criterion_group, criterion_main};
use pdf_maldives_be::pdf_inference::{
    init_pdfium,
    infer_pdf_semantics,
    reconstruct::{extract_fragments, group_into_lines, merge_into_blocks, reconstruct_page},
};

fn load_test_pdf_bytes() -> Vec<u8> {
    fs::read("test/qemu_long_pdf.pdf").unwrap()
}

fn bench_pdf_pipeline(c: &mut Criterion) {
    let pdf_bytes = load_test_pdf_bytes();
    let pdfium = init_pdfium();
    let document = pdfium.load_pdf_from_byte_slice(&pdf_bytes, None).unwrap();
    let page = document.pages().get(0).unwrap();

    let mut group = c.benchmark_group("pdf_pipeline");

    group.bench_function("1_extract_fragments", |b| {
        b.iter(|| {
            black_box(extract_fragments(black_box(&page)))
        });
    });

    let (fragments, _underlines) = extract_fragments(&page);
    let page_width = page.width().value;

    group.bench_function("2_group_into_lines", |b| {
        b.iter(|| {
            black_box(group_into_lines(black_box(fragments.clone()), black_box(page_width)))
        });
    });


    let lines = group_into_lines(fragments.clone(), page_width);

    group.bench_function("3_merge_into_blocks", |b| {
        b.iter(|| {
            black_box(merge_into_blocks(black_box(lines.clone())))
        });
    });

    group.bench_function("4_full_page_reconstruct", |b| {
        b.iter(|| {
            black_box(reconstruct_page(black_box(&page)))
        });
    });

    group.finish();

    // Complete benchmark
    c.bench_function("full_document_infer", |b| {
        b.iter(|| {
            black_box(infer_pdf_semantics(black_box(&pdf_bytes)))
        });
    });
}

fn custom_criterion() -> Criterion {
    Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(500))
        .sample_size(10)
}

// I can also do this as criterion(benches, bench_pdf_parsing); if I were not using custom config
criterion_group! {
    name = benches;
    config = custom_criterion();
    targets = bench_pdf_pipeline
}
criterion_main!(benches);