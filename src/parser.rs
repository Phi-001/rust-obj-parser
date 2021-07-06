use std::cmp;
use std::collections::HashSet;
use std::error::Error;
use std::sync::mpsc;
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
        |keyword, args, obj_vertex_data, _| match keyword {
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
        |keyword, args, obj_vertex_data, gl_vertex_data| {
            if keyword == "f" {
                face(args, obj_vertex_data, gl_vertex_data).unwrap()
            }
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
    T: Fn(&str, std::str::SplitWhitespace, &mut ObjectInfo, &mut VertexData)
        + 'static
        + Send
        + Copy,
    for<'a> U: Fn(Vec<&'a str>, Vec<&'a str>) -> Vec<&'a str> + 'static + Send + Copy,
{
    let (tx, rx) = mpsc::channel();

    let mut handles = Vec::new();

    for id in 0..NUM_CORES {
        let tx = tx.clone();
        let obj_file = obj_file.clone();
        let mut obj_vertex_data = state.obj_vertex_data.clone();
        let mut gl_vertex_data = state.gl_vertex_data.clone();
        handles.push(thread::spawn(move || {
            let lines = obj_file.lines();
            let (index, vertex): (Vec<&str>, Vec<&str>) =
                lines.partition(|line| line.starts_with('f'));

            let data = lines_extractor(index, vertex);

            let chunk_size = data.len() / NUM_CORES + 1;
            let start = id * chunk_size;
            let end = cmp::min((id + 1) * chunk_size, data.len());

            let partioned_lines = &data[start..end];

            for &line in partioned_lines {
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }

                let mut parts = line.split_whitespace();
                let keyword = parts.next().unwrap();

                line_handler(keyword, parts, &mut obj_vertex_data, &mut gl_vertex_data);
            }

            tx.send(Message {
                content: State {
                    obj_vertex_data,
                    gl_vertex_data,
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

    for message in rx {
        messages.push(message);
    }

    messages.sort_by(|a, b| a.id.cmp(&b.id));

    let mut obj_vertex_data = state.obj_vertex_data;

    let mut gl_vertex_data = state.gl_vertex_data;

    for message in messages {
        let mut iter = message.content.obj_vertex_data.position.into_iter();
        iter.next();
        obj_vertex_data.position.extend(iter);
        let mut iter = message.content.obj_vertex_data.normal.into_iter();
        iter.next();
        obj_vertex_data.normal.extend(iter);
        let mut iter = message.content.obj_vertex_data.texcoord.into_iter();
        iter.next();
        obj_vertex_data.texcoord.extend(iter);
        gl_vertex_data
            .position
            .extend(message.content.gl_vertex_data.position);
        gl_vertex_data
            .normal
            .extend(message.content.gl_vertex_data.normal);
        gl_vertex_data
            .texcoord
            .extend(message.content.gl_vertex_data.texcoord);
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

#[derive(Clone)]
pub struct Vec2 {
    x: f64,
    y: f64,
}

impl Vec2 {
    fn new() -> Vec2 {
        Vec2 { x: 0f64, y: 0f64 }
    }
}

#[derive(Clone)]
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
