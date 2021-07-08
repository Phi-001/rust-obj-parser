use criterion::{criterion_group, criterion_main, Criterion};
use std::cmp;
use std::fs;

fn partion_cases(c: &mut Criterion) {
    let obj_file = fs::read_to_string("al.obj").unwrap();

    let mut group = c.benchmark_group("Partions");
    group.bench_function("partion naive", |b| {
        b.iter(|| {
            let lines = obj_file.lines();
            let (index, _): (Vec<&str>, Vec<&str>) = lines.partition(|line| line.starts_with('f'));

            let data = index;

            let chunk_size = data.len() / 4 + 1;
            let start = 1 * chunk_size;
            let end = cmp::min((1 + 1) * chunk_size, data.len());

            let partioned_lines = &data[start..end];

            partioned_lines.iter().for_each(drop);
        })
    });
    group.bench_function("partion new", |b| {
        b.iter(|| {
            let split_index = obj_file.rfind("\nv").unwrap() + 1;
            let (_, split) = obj_file.split_at(split_index);
            let split_index = split_index + split.find('\n').unwrap();
            let (vertex, _) = obj_file.split_at(split_index);
            let chunk_size = split_index / 4 + 1;
            vertex
                .lines()
                .skip(chunk_size * 1)
                .take(chunk_size)
                .for_each(drop);
        })
    });
    group.finish();
}

criterion_group!(benches, partion_cases);
criterion_main!(benches);
