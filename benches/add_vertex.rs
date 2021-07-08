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
        .extend(obj_vertex_data.position[obj_index].to_arr());

    if let Some(obj_index) = iter.next() {
        let obj_index: usize = obj_index.parse()?;
        gl_vertex_data
            .texcoord
            .extend(obj_vertex_data.texcoord[obj_index].to_arr());
    }

    if let Some(obj_index) = iter.next() {
        let obj_index: usize = obj_index.parse()?;
        gl_vertex_data
            .normal
            .extend(obj_vertex_data.normal[obj_index].to_arr());
    }

    Ok(())
}

fn add_vertex_cases(c: &mut Criterion) {
    let mut group = c.benchmark_group("Add vertex");
    let obj_info = ObjectInfo {
        position: vec![
            Vec3::new(),
            Vec3 {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
        ],
        texcoord: vec![Vec2::new()],
        normal: vec![Vec3::new()],
    };
    group.bench_function("add_vertex naive", |b| {
        b.iter(|| {
            add_vertex(
                black_box("1"),
                black_box(&obj_info),
                black_box(&mut VertexData {
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
pub struct Vec3 {
    x: f64,
    y: f64,
    z: f64,
}

impl Vec3 {
    fn new() -> Vec3 {
        Vec3 {
            x: 0f64,
            y: 0f64,
            z: 0f64,
        }
    }

    fn to_arr(&self) -> [f64; 3] {
        [self.x, self.x, self.z]
    }
}

#[derive(Clone, Debug)]
pub struct Vec2 {
    x: f64,
    y: f64,
}

impl Vec2 {
    fn new() -> Vec2 {
        Vec2 { x: 0f64, y: 0f64 }
    }

    fn to_arr(&self) -> [f64; 2] {
        [self.x, self.x]
    }
}

#[derive(Clone, Debug)]
struct ObjectInfo {
    position: Vec<Vec3>,
    texcoord: Vec<Vec2>,
    normal: Vec<Vec3>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VertexData {
    pub position: Vec<f64>,
    pub texcoord: Vec<f64>,
    pub normal: Vec<f64>,
}
