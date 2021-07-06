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
        if line == "" || line.starts_with("#") {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        let keyword = parts[0];
        let args = parts[1..].to_vec();

        match keyword {
            "v" => vertex(args, &mut obj_vertex_data)?,
            "vn" => vertex_normal(args, &mut obj_vertex_data)?,
            "vt" => vertex_texture(args, &mut obj_vertex_data)?,
            "f" => face(args, &obj_vertex_data, &mut gl_vertex_data)?,
            _ => {
                unhandled_keywords.insert(keyword);
            }
        }
    }

    println!("Unhandled keywords: {:?}", unhandled_keywords);

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
            _ => (),
        },
        |obj_file| {
            let lines = obj_file.lines().map(|string| String::from(string));
            let (_, data): (_, Vec<String>) = lines.partition(|line| line.starts_with("f"));
            data.chunks(data.len() / NUM_CORES)
                .map(|arr| arr.to_vec())
                .collect()
        },
        state,
    );

    let state = create_thread_parse(
        obj_file.clone(),
        |keyword, args, obj_vertex_data, gl_vertex_data| match keyword {
            "f" => face(args, obj_vertex_data, gl_vertex_data).unwrap(),
            _ => (),
        },
        |obj_file| {
            let lines = obj_file.lines().map(|string| String::from(string));
            let (index, _): (Vec<String>, _) = lines.partition(|line| line.starts_with("f"));
            index
                .chunks(index.len() / NUM_CORES)
                .map(|arr| arr.to_vec())
                .collect()
        },
        state,
    );

    Ok(state.gl_vertex_data)
}

fn create_thread_parse<'a, T, U>(
    obj_file: String,
    line_handler: T,
    lines_extractor: U,
    state: State<ObjectInfo, VertexData>,
) -> State<ObjectInfo, VertexData>
where
    T: Fn(&str, Vec<&str>, &mut ObjectInfo, &mut VertexData) -> () + 'static + Send + Copy,
    U: Fn(String) -> Vec<Vec<String>> + 'static + Send + Copy,
{
    let (tx, rx) = mpsc::channel();

    let mut handles = Vec::new();

    for id in 0..NUM_CORES {
        let tx = tx.clone();
        let obj_file = obj_file.clone();
        let mut obj_vertex_data = state.obj_vertex_data.clone();
        let mut gl_vertex_data = state.gl_vertex_data.clone();
        handles.push(thread::spawn(move || {
            let data = lines_extractor(obj_file);
            let partioned_lines = &data[id];

            for line in partioned_lines {
                if line == "" || line.starts_with("#") {
                    continue;
                }

                let parts: Vec<&str> = line.split_whitespace().collect();
                let keyword = parts[0];
                let args = parts[1..].to_vec();

                line_handler(keyword, args, &mut obj_vertex_data, &mut gl_vertex_data);
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
        obj_vertex_data
            .position
            .extend(message.content.obj_vertex_data.position);
        obj_vertex_data
            .normal
            .extend(message.content.obj_vertex_data.normal);
        obj_vertex_data
            .texcoord
            .extend(message.content.obj_vertex_data.texcoord);
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

fn vertex(args: Vec<&str>, obj_vertex_data: &mut ObjectInfo) -> Result<(), Box<dyn Error>> {
    obj_vertex_data.position.push(Vec3 {
        x: args[0].parse()?,
        y: args[1].parse()?,
        z: args[2].parse()?,
    });

    Ok(())
}

fn vertex_normal(args: Vec<&str>, obj_vertex_data: &mut ObjectInfo) -> Result<(), Box<dyn Error>> {
    obj_vertex_data.normal.push(Vec3 {
        x: args[0].parse()?,
        y: args[1].parse()?,
        z: args[2].parse()?,
    });

    Ok(())
}

fn vertex_texture(args: Vec<&str>, obj_vertex_data: &mut ObjectInfo) -> Result<(), Box<dyn Error>> {
    obj_vertex_data.texcoord.push(Vec2 {
        x: args[0].parse()?,
        y: args[1].parse()?,
    });

    Ok(())
}

fn face(
    args: Vec<&str>,
    obj_vertex_data: &ObjectInfo,
    gl_vertex_data: &mut VertexData,
) -> Result<(), Box<dyn Error>> {
    for tri in 0..args.len() - 2 {
        add_vertex(args[0], obj_vertex_data, gl_vertex_data)?;
        add_vertex(args[tri + 1], obj_vertex_data, gl_vertex_data)?;
        add_vertex(args[tri + 2], obj_vertex_data, gl_vertex_data)?;
    }

    Ok(())
}

fn add_vertex(
    vert: &str,
    obj_vertex_data: &ObjectInfo,
    gl_vertex_data: &mut VertexData,
) -> Result<(), Box<dyn Error>> {
    for (i, obj_index) in vert.split("/").enumerate() {
        if obj_index == "" {
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

#[derive(Clone)]
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
