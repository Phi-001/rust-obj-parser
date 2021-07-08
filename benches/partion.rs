use criterion::{criterion_group, criterion_main, Criterion};
use std::fs;

fn partion_cases(c: &mut Criterion) {
    let obj_file = fs::read_to_string("al.obj").unwrap();

    let mut group = c.benchmark_group("Partions");
    group.bench_function("partion naive", |b| {
        b.iter(|| {
            let lines = obj_file.lines();
            let (index, vertex): (Vec<&str>, Vec<&str>) =
                lines.partition(|line| line.starts_with('f'));
            (index, vertex)
        })
    });
    group.bench_function("partion new", |b| {
        b.iter(|| {
            let vertex = obj_file
                .split_at(obj_file.rfind("\nf").unwrap() + 1)
                .0
                .lines();
            vertex
        })
    });
    group.finish();
}

criterion_group!(benches, partion_cases);
criterion_main!(benches);
