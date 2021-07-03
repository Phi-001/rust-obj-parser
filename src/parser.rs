use std::error::Error;
use std::collections::HashSet;

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

pub struct Vec2 {
    x: f64,
    y: f64,
}

impl Vec2 {
    fn new() -> Vec2 {
        Vec2 {
            x: 0f64,
            y: 0f64,
        }
    }
}

struct ObjectInfo {
    position: Vec<Vec3>,
    texcoord: Vec<Vec2>,
    normal: Vec<Vec3>,
}

pub struct VertexData {
    pub position: Vec<f64>,
    pub texcoord: Vec<f64>,
    pub normal: Vec<f64>,
}

pub fn parse_obj(obj_file_string: String) -> Result<VertexData, Box<dyn Error>> {
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

    for line in obj_file_string.lines() {
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
            },
        }
    }

    println!("Unhandled keywords: {:?}", unhandled_keywords);

    Ok(gl_vertex_data)
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

fn face(args: Vec<&str>, obj_vertex_data: &ObjectInfo, gl_vertex_data: &mut VertexData) -> Result<(), Box<dyn Error>> {
    for tri in 0..args.len() - 2 {
        add_vertex(args[0], obj_vertex_data, gl_vertex_data)?;
        add_vertex(args[tri + 1], obj_vertex_data, gl_vertex_data)?;
        add_vertex(args[tri + 2], obj_vertex_data, gl_vertex_data)?;
    }

    Ok(())
}

fn add_vertex(vert: &str, obj_vertex_data: &ObjectInfo, gl_vertex_data: &mut VertexData) -> Result<(), Box<dyn Error>> {
    let ptn = vert.split("/").enumerate();
    for (i, obj_index_str) in ptn {
        if obj_index_str == "" {
            continue;
        }

        let obj_index: usize = obj_index_str.parse()?;
        match i {
            0 => {
                let vec3 = &obj_vertex_data.position[obj_index];
                gl_vertex_data.position.extend([vec3.x, vec3.y, vec3.z].iter());
            },
            1 => {
                let vec2 = &obj_vertex_data.texcoord[obj_index];
                gl_vertex_data.texcoord.extend([vec2.x, vec2.y].iter());
            },
            2 => {
                let vec3 = &obj_vertex_data.normal[obj_index];
                gl_vertex_data.normal.extend([vec3.x, vec3.y, vec3.z].iter());
            }
            _ => ()
        }
    }

    Ok(())
}