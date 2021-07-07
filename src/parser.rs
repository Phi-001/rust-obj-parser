use std::cmp;
use std::collections::HashSet;
use std::error::Error;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

const NUM_CORES: usize = 4;

pub fn _parse_obj(obj_file: String) -> Result<VertexData, Box<dyn Error>> {
    let mut obj_vertex_data = ObjectInfo {
        position: vec![Vec3::new()],
        texcoord: vec![Vec2::new()],
        normal: vec![Vec3::new()],
    };

    let mut gl_vertex_data = VertexData {
        position: vec![],
        texcoord: vec![],
        normal: vec![],
    };

    let mut unhandled_keywords = HashSet::new();

    for line in obj_file.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let keyword = parts.next().unwrap();
        let args = parts;

        match keyword {
            "v" => vertex(args, &mut obj_vertex_data)?,
            "vn" => vertex_normal(args, &mut obj_vertex_data)?,
            "vt" => vertex_texture(args, &mut obj_vertex_data)?,
            "f" => face(args, &obj_vertex_data, &mut gl_vertex_data)?,
            "g" => (),
            "s" => (),
            "usemtl" => (),
            "mtllib" => (),
            _ => {
                unhandled_keywords.insert(keyword);
            }
        }
    }

    if !unhandled_keywords.is_empty() {
        println!("Unhandled keywords: {:?}", unhandled_keywords);
    }

    Ok(gl_vertex_data)
}

pub fn parse_obj_threaded(obj_file: String) -> Result<VertexData, Box<dyn Error>> {
    let state = State {
        obj_vertex_data: ObjectInfo {
            position: vec![Vec3::new()],
            texcoord: vec![Vec2::new()],
            normal: vec![Vec3::new()],
        },
        gl_vertex_data: VertexData {
            position: vec![],
            texcoord: vec![],
            normal: vec![],
        },
    };

    let state = create_thread_parse(
        obj_file.clone(),
        |keyword, args, _, _, obj_vertex_data, _| match keyword {
            "v" => vertex(args, obj_vertex_data).unwrap(),
            "vn" => vertex_normal(args, obj_vertex_data).unwrap(),
            "vt" => vertex_texture(args, obj_vertex_data).unwrap(),
            "g" => (),
            "s" => (),
            "usemtl" => (),
            "mtllib" => (),
            _ => println!("unhandled keyword: {}", keyword),
        },
        |_, vertex| vertex,
        state,
    );

    let state = create_thread_parse(
        obj_file,
        |_, args, obj_vertex_data, _, _, gl_vertex_data| {
            face(args, obj_vertex_data, gl_vertex_data).unwrap()
        },
        |index, _| index,
        state,
    );

    Ok(state.gl_vertex_data)
}

fn create_thread_parse<T, U>(
    obj_file: String,
    line_handler: T,
    lines_extractor: U,
    state: State<ObjectInfo, VertexData>,
) -> State<ObjectInfo, VertexData>
where
    T: Fn(
            &str,
            std::str::SplitWhitespace,
            &ObjectInfo,
            &VertexData,
            &mut ObjectInfo,
            &mut VertexData,
        )
        + 'static
        + Send
        + Copy,
    for<'a> U: Fn(Vec<&'a str>, Vec<&'a str>) -> Vec<&'a str> + 'static + Send + Copy,
{
    let (tx, rx) = mpsc::channel();

    let mut handles = Vec::new();

    let obj_vertex_data = Arc::new(state.obj_vertex_data);
    let gl_vertex_data = Arc::new(state.gl_vertex_data);
    let obj_file = Arc::new(obj_file);

    for id in 0..NUM_CORES {
        let tx = tx.clone();
        let obj_file = Arc::clone(&obj_file);
        let obj_vertex_data_read = Arc::clone(&obj_vertex_data);
        let gl_vertex_data_read = Arc::clone(&gl_vertex_data);
        handles.push(thread::spawn(move || {
            let lines = obj_file.lines();
            let (index, vertex): (Vec<&str>, Vec<&str>) =
                lines.partition(|line| line.starts_with('f'));

            let data = lines_extractor(index, vertex);

            let chunk_size = data.len() / NUM_CORES + 1;
            let start = id * chunk_size;
            let end = cmp::min((id + 1) * chunk_size, data.len());

            let partioned_lines = &data[start..end];

            let mut obj_vertex_data_write = ObjectInfo {
                position: vec![],
                texcoord: vec![],
                normal: vec![],
            };

            let mut gl_vertex_data_write = VertexData {
                position: vec![],
                texcoord: vec![],
                normal: vec![],
            };

            for &line in partioned_lines {
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }

                let mut parts = line.split_whitespace();
                let keyword = parts.next().unwrap();

                line_handler(
                    keyword,
                    parts,
                    &obj_vertex_data_read,
                    &gl_vertex_data_read,
                    &mut obj_vertex_data_write,
                    &mut gl_vertex_data_write,
                );
            }

            tx.send(Message {
                content: State {
                    obj_vertex_data: obj_vertex_data_write,
                    gl_vertex_data: gl_vertex_data_write,
                },
                id,
            })
            .unwrap();
        }));
    }

    drop(tx);

    for handle in handles {
        handle.join().unwrap();
    }

    let mut messages = Vec::new();

    let mut obj_vertex_data = Arc::try_unwrap(obj_vertex_data).unwrap();
    let mut gl_vertex_data = Arc::try_unwrap(gl_vertex_data).unwrap();

    let mut obj_reserve = ObjectInfoReserve::new();
    let mut gl_reserve = VertexDataReserve::new();

    for message in rx {
        let content = &message.content;
        obj_reserve.reserve(&content.obj_vertex_data);
        gl_reserve.reserve(&content.gl_vertex_data);
        messages.push(message);
    }

    obj_vertex_data.reserve(obj_reserve);
    gl_vertex_data.reserve(gl_reserve);

    messages.sort_by(|a, b| a.id.cmp(&b.id));

    for message in messages {
        let content = message.content;
        obj_vertex_data.extend(content.obj_vertex_data);
        gl_vertex_data.extend(content.gl_vertex_data);
    }

    State {
        obj_vertex_data,
        gl_vertex_data,
    }
}

struct Message<T, U> {
    content: State<T, U>,
    id: usize,
}

struct State<T, U> {
    obj_vertex_data: T,
    gl_vertex_data: U,
}

fn vertex(
    mut args: std::str::SplitWhitespace,
    obj_vertex_data: &mut ObjectInfo,
) -> Result<(), Box<dyn Error>> {
    obj_vertex_data.position.push(Vec3 {
        x: args.next().unwrap().parse()?,
        y: args.next().unwrap().parse()?,
        z: args.next().unwrap().parse()?,
    });

    Ok(())
}

fn vertex_normal(
    mut args: std::str::SplitWhitespace,
    obj_vertex_data: &mut ObjectInfo,
) -> Result<(), Box<dyn Error>> {
    obj_vertex_data.normal.push(Vec3 {
        x: args.next().unwrap().parse()?,
        y: args.next().unwrap().parse()?,
        z: args.next().unwrap().parse()?,
    });

    Ok(())
}

fn vertex_texture(
    mut args: std::str::SplitWhitespace,
    obj_vertex_data: &mut ObjectInfo,
) -> Result<(), Box<dyn Error>> {
    obj_vertex_data.texcoord.push(Vec2 {
        x: args.next().unwrap().parse()?,
        y: args.next().unwrap().parse()?,
    });

    Ok(())
}

fn face(
    mut args: std::str::SplitWhitespace,
    obj_vertex_data: &ObjectInfo,
    gl_vertex_data: &mut VertexData,
) -> Result<(), Box<dyn Error>> {
    let first = args.next().unwrap();

    let mut second = args.next().unwrap();
    let mut third;

    for vertex in args {
        third = vertex;
        add_vertex(first, obj_vertex_data, gl_vertex_data)?;
        add_vertex(second, obj_vertex_data, gl_vertex_data)?;
        add_vertex(third, obj_vertex_data, gl_vertex_data)?;
        second = third;
    }

    Ok(())
}

fn add_vertex(
    vert: &str,
    obj_vertex_data: &ObjectInfo,
    gl_vertex_data: &mut VertexData,
) -> Result<(), Box<dyn Error>> {
    for (i, obj_index) in vert.split('/').enumerate() {
        if obj_index.is_empty() {
            continue;
        }

        let obj_index: usize = obj_index.parse()?;
        match i {
            0 => {
                let vec3 = &obj_vertex_data.position[obj_index];
                gl_vertex_data.position.extend([vec3.x, vec3.y, vec3.z]);
            }
            1 => {
                let vec2 = &obj_vertex_data.texcoord[obj_index];
                gl_vertex_data.texcoord.extend([vec2.x, vec2.y]);
            }
            2 => {
                let vec3 = &obj_vertex_data.normal[obj_index];
                gl_vertex_data.normal.extend([vec3.x, vec3.y, vec3.z]);
            }
            _ => (),
        }
    }

    Ok(())
}

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
}

#[derive(Clone, Debug)]
struct ObjectInfo {
    position: Vec<Vec3>,
    texcoord: Vec<Vec2>,
    normal: Vec<Vec3>,
}

impl ObjectInfo {
    fn extend(&mut self, info: ObjectInfo) {
        self.position.extend(info.position);
        self.normal.extend(info.normal);
        self.texcoord.extend(info.texcoord);
    }

    fn reserve(&mut self, reserve: ObjectInfoReserve) {
        self.position.reserve(reserve.position);
        self.normal.reserve(reserve.normal);
        self.texcoord.reserve(reserve.texcoord);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VertexData {
    pub position: Vec<f64>,
    pub texcoord: Vec<f64>,
    pub normal: Vec<f64>,
}

impl VertexData {
    fn extend(&mut self, info: VertexData) {
        self.position.extend(info.position);
        self.normal.extend(info.normal);
        self.texcoord.extend(info.texcoord);
    }

    fn reserve(&mut self, reserve: VertexDataReserve) {
        self.position.reserve(reserve.position);
        self.normal.reserve(reserve.normal);
        self.texcoord.reserve(reserve.texcoord);
    }
}

struct ObjectInfoReserve {
    position: usize,
    texcoord: usize,
    normal: usize,
}

impl ObjectInfoReserve {
    fn reserve(&mut self, info: &ObjectInfo) {
        self.position += info.position.len();
        self.normal += info.normal.len();
        self.texcoord += info.texcoord.len();
    }

    fn new() -> Self {
        ObjectInfoReserve {
            position: 0,
            texcoord: 0,
            normal: 0,
        }
    }
}

struct VertexDataReserve {
    position: usize,
    texcoord: usize,
    normal: usize,
}

impl VertexDataReserve {
    fn reserve(&mut self, info: &VertexData) {
        self.position += info.position.len();
        self.normal += info.normal.len();
        self.texcoord += info.texcoord.len();
    }

    fn new() -> Self {
        VertexDataReserve {
            position: 0,
            texcoord: 0,
            normal: 0,
        }
    }
}
