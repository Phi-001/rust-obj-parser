use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::error::Error;

fn add_vertex(
    vert: &str,
    obj_vertex_data: &ObjectInfo,
    gl_vertex_data: &mut VertexData,
) -> Result<(), Box<dyn Error>> {
    let mut iter = vert.split('/');

    let obj_index = iter.next().unwrap();
    let obj_index: usize = obj_index.parse()?;
    gl_vertex_data
        .position
        .extend(obj_vertex_data.position[obj_index]);

    if let Some(obj_index) = iter.next() {
        let obj_index: usize = obj_index.parse()?;
        gl_vertex_data
            .texcoord
            .extend(obj_vertex_data.texcoord[obj_index]);
    }

    if let Some(obj_index) = iter.next() {
        let obj_index: usize = obj_index.parse()?;
        gl_vertex_data
            .normal
            .extend(obj_vertex_data.normal[obj_index]);
    }

    Ok(())
}

fn add_vertex_test(
    vert: &str,
    obj_vertex_data: &ObjectInfoTest,
    gl_vertex_data: &mut ObjectInfoTest,
) -> Result<(), Box<dyn Error>> {
    let mut iter = vert.split('/');

    let obj_index = iter.next().unwrap();
    let obj_index: usize = obj_index.parse()?;
    gl_vertex_data
        .position
        .extend(&obj_vertex_data.position[(obj_index * 3)..((obj_index + 1) * 3)]);

    if let Some(obj_index) = iter.next() {
        let obj_index: usize = obj_index.parse()?;
        gl_vertex_data
            .texcoord
            .extend(&obj_vertex_data.position[(obj_index * 2)..((obj_index + 1) * 2)]);
    }

    if let Some(obj_index) = iter.next() {
        let obj_index: usize = obj_index.parse()?;
        gl_vertex_data
            .normal
            .extend(&obj_vertex_data.position[(obj_index * 3)..((obj_index + 1) * 3)]);
    }

    Ok(())
}

fn add_vertex_cases(c: &mut Criterion) {
    let mut group = c.benchmark_group("Add vertex");
    let obj_info = ObjectInfo {
        position: vec![[0.0; 3], [1.0; 3]],
        texcoord: vec![[0.0; 2], [1.0; 2]],
        normal: vec![[0.0; 3], [1.0; 3]],
    };
    let obj_info_test = ObjectInfoTest {
        position: vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
        texcoord: vec![0.0, 0.0, 1.0, 1.0],
        normal: vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
    };
    group.bench_function("add_vertex naive", |b| {
        b.iter(|| {
            add_vertex(
                black_box("1"),
                &obj_info,
                black_box(&mut VertexData {
                    position: vec![],
                    normal: vec![],
                    texcoord: vec![],
                }),
            )
        })
    });
    group.bench_function("add_vertex testing", |b| {
        b.iter(|| {
            add_vertex_test(
                black_box("1"),
                &obj_info_test,
                black_box(&mut ObjectInfoTest {
                    position: vec![],
                    normal: vec![],
                    texcoord: vec![],
                }),
            )
        })
    });
    group.finish();
}

criterion_group!(benches, add_vertex_cases);
criterion_main!(benches);

#[derive(Clone, Debug)]
struct ObjectInfo {
    position: Vec<[f32; 3]>,
    texcoord: Vec<[f32; 2]>,
    normal: Vec<[f32; 3]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VertexData {
    pub position: Vec<f32>,
    pub texcoord: Vec<f32>,
    pub normal: Vec<f32>,
}

#[derive(Clone, Debug)]
struct ObjectInfoTest {
    position: Vec<f32>,
    texcoord: Vec<f32>,
    normal: Vec<f32>,
}
