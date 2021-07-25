use criterion::{criterion_group, criterion_main, Criterion};
use std::fs;

extern crate rust_obj_parser;

use rust_obj_parser::parser;

fn bench_threaded_vs_non_threaded(c: &mut Criterion) {
    let content = fs::read_to_string("al.obj").unwrap();

    c.bench_function("parallel parser", |b| {
        b.iter(|| parser::parse_obj_threaded(content.clone()))
    });
}

criterion_group!(benches, bench_threaded_vs_non_threaded);
criterion_main!(benches);
