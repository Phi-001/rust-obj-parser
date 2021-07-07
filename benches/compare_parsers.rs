use criterion::{criterion_group, criterion_main, Criterion};
use std::fs;

#[path = "../src/parser.rs"]
mod parser;

fn bench_threaded_vs_non_threaded(c: &mut Criterion) {
    let content = fs::read_to_string("al.obj").unwrap();

    let mut group = c.benchmark_group("Parsers");
    group.bench_function("multiple thread", |b| {
        b.iter(|| parser::parse_obj_threaded(content.clone()))
    });
    group.bench_function("single thread", |b| {
        b.iter(|| parser::_parse_obj(content.clone()))
    });
    group.finish();
}

criterion_group!(benches, bench_threaded_vs_non_threaded);
criterion_main!(benches);
