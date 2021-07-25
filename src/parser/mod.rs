use std::error::Error;
use std::sync::mpsc;
use std::sync::Arc;

mod thread_pool;

use thread_pool::ThreadPool;

const NUM_CORES: usize = 4;

pub fn parse_obj_threaded(obj_file: String) -> Result<VertexDataGrouped, Box<dyn Error>> {
    let obj_file = Arc::new(obj_file);

    let thread_pool = ThreadPool::new(NUM_CORES);

    let (index_str, vertex_str) = extract_vertices_and_indices(Arc::clone(&obj_file), &thread_pool);

    let state = State {
        obj_vertex_data: ObjectInfo {
            position: vec![[0.0; 3]],
            texcoord: vec![[0.0; 2]],
            normal: vec![[0.0; 3]],
        },
        gl_vertex_data: VertexDataGrouped { groups: vec![] },
    };

    let state = create_parse_thread(
        vertex_str,
        Arc::clone(&obj_file),
        |keyword, args, _, _, obj_vertex_data, gl_vertex_data| {
            (
                match keyword {
                    "v" => vertex(args, obj_vertex_data).unwrap(),
                    "vn" => vertex_normal(args, obj_vertex_data).unwrap(),
                    "vt" => vertex_texture(args, obj_vertex_data).unwrap(),
                    "s" => obj_vertex_data,
                    "usemtl" => obj_vertex_data,
                    "mtllib" => obj_vertex_data,
                    "#" => obj_vertex_data,
                    _ => {
                        println!("unhandled keyword: {}", keyword);
                        obj_vertex_data
                    }
                },
                gl_vertex_data,
            )
        },
        state,
        &thread_pool,
    );

    let state = create_parse_thread(
        index_str,
        Arc::clone(&obj_file),
        |keyword, args, obj_vertex_data, _, obj_vertex_data_write, gl_vertex_data| {
            (
                obj_vertex_data_write,
                match keyword {
                    "f" => face(args, obj_vertex_data, gl_vertex_data).unwrap(),
                    "g" => group(args, gl_vertex_data),
                    _ => gl_vertex_data,
                },
            )
        },
        state,
        &thread_pool,
    );

    Ok(state.gl_vertex_data)
}

type IndexVertex = (Vec<Vec<(usize, usize)>>, Vec<Vec<(usize, usize)>>);

fn extract_vertices_and_indices(obj_file: Arc<String>, thread_pool: &ThreadPool) -> IndexVertex {
    let obj_file = Arc::new(obj_file);
    let len = obj_file.len();
    let chunk_size = len / NUM_CORES + 1;

    let (tx, rx) = mpsc::channel();

    for id in 0..NUM_CORES {
        let tx = tx.clone();
        let obj_file = Arc::clone(&obj_file);
        thread_pool.execute(
            Box::new(move || {
                let left_split_index = {
                    let (_, right) = obj_file.split_at(id * chunk_size);
                    id * chunk_size + right.find('\n').unwrap()
                };

                let right_split_index = {
                    if id == NUM_CORES - 1 {
                        len
                    } else {
                        let (_, right) = obj_file.split_at((id + 1) * chunk_size);
                        (id + 1) * chunk_size + right.find('\n').unwrap()
                    }
                };

                let chunk = &obj_file[left_split_index..right_split_index];

                let (index_str, vertex_str, _) = chunk.split_inclusive('\n').fold(
                    (
                        Vec::with_capacity(chunk.len() / 30),
                        Vec::with_capacity(chunk.len() / 30),
                        left_split_index,
                    ),
                    |(mut index, mut vertex, location), line| {
                        let new_location = location + line.len();

                        if line.starts_with('f') || line.starts_with('g') {
                            index.push((location, new_location));
                        } else {
                            vertex.push((location, new_location));
                        }

                        (index, vertex, new_location)
                    },
                );

                tx.send(Message {
                    content: (index_str, vertex_str),
                    id,
                })
                .unwrap()
            }),
            id,
        );
    }

    drop(tx);

    let mut messages = Vec::with_capacity(NUM_CORES);

    let mut index_str_len = 0;
    let mut vertex_str_len = 0;

    for message in rx {
        let (index_str, vertex_str) = &message.content;
        index_str_len += index_str.len();
        vertex_str_len += vertex_str.len();
        messages.push(message);
    }

    messages.sort_by(|a, b| a.id.cmp(&b.id));

    let mut index_str = Vec::with_capacity(NUM_CORES);
    let mut vertex_str = Vec::with_capacity(NUM_CORES);

    let index_chunk_size = index_str_len / NUM_CORES + 1;
    let vertex_chunk_size = vertex_str_len / NUM_CORES + 1;

    index_str.push(Vec::with_capacity(index_chunk_size));
    vertex_str.push(Vec::with_capacity(vertex_chunk_size));

    for message in messages {
        let (index, vertex) = message.content;

        extend_fit(&mut index_str, index, index_chunk_size);
        extend_fit(&mut vertex_str, vertex, vertex_chunk_size);
    }

    (index_str, vertex_str)
}

fn extend_fit(
    data: &mut Vec<Vec<(usize, usize)>>,
    extend_data: std::vec::Vec<(usize, usize)>,
    fit_size: usize,
) {
    let mut last_index = data.last_mut().unwrap();
    let mut space_left = fit_size - last_index.len();

    let mut current_index = 0;

    while space_left < extend_data.len() - current_index {
        last_index.extend(&extend_data[current_index..current_index + space_left]);
        current_index += space_left;

        data.push(Vec::with_capacity(fit_size));
        last_index = data.last_mut().unwrap();
        space_left = fit_size;
    }

    last_index.extend(&extend_data[current_index..]);
}

fn create_parse_thread<T>(
    data: Vec<Vec<(usize, usize)>>,
    obj_file: Arc<String>,
    line_handler: T,
    state: State<ObjectInfo, VertexDataGrouped>,
    thread_pool: &ThreadPool,
) -> State<ObjectInfo, VertexDataGrouped>
where
    T: Fn(
            &str,
            std::str::SplitWhitespace<'_>,
            &ObjectInfo,
            &VertexDataGrouped,
            ObjectInfo,
            VertexDataGroupedThread,
        ) -> (ObjectInfo, VertexDataGroupedThread)
        + 'static
        + Send
        + Copy,
{
    let (tx, rx) = mpsc::channel();

    let state = Arc::new(state);
    let data = Arc::new(data);

    for id in 0..NUM_CORES {
        let tx = tx.clone();
        let data = Arc::clone(&data);
        let state = Arc::clone(&state);
        let obj_file = Arc::clone(&obj_file);
        thread_pool.execute(
            Box::new(move || {
                let mut obj_vertex_data = ObjectInfo {
                    position: vec![],
                    texcoord: vec![],
                    normal: vec![],
                };

                let mut gl_vertex_data = VertexDataGroupedThread::new();

                for &(start, end) in data[id].iter() {
                    let line = &obj_file[start..end].trim();

                    if line.is_empty() {
                        continue;
                    }

                    let mut parts = line.split_whitespace();
                    let keyword = parts.next().unwrap();
                    let (obj_vertex_data_new, gl_vertex_data_new) = line_handler(
                        keyword,
                        parts,
                        &state.obj_vertex_data,
                        &state.gl_vertex_data,
                        obj_vertex_data,
                        gl_vertex_data,
                    );

                    obj_vertex_data = obj_vertex_data_new;
                    gl_vertex_data = gl_vertex_data_new;
                }

                tx.send(Message {
                    content: State {
                        obj_vertex_data,
                        gl_vertex_data,
                    },
                    id,
                })
                .unwrap();
            }),
            id,
        );
    }

    drop(tx);

    let mut messages = Vec::new();

    let mut obj_reserve = ObjectInfoReserve::new();

    for message in rx {
        let content = &message.content;
        obj_reserve.reserve(&content.obj_vertex_data);
        messages.push(message);
    }

    let state = Arc::try_unwrap(state).unwrap();
    let mut obj_vertex_data = state.obj_vertex_data;
    let mut gl_vertex_data = state.gl_vertex_data;

    obj_vertex_data.reserve(obj_reserve);

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

struct Message<T> {
    content: T,
    id: usize,
}

#[derive(Debug)]
struct State<T, U> {
    obj_vertex_data: T,
    gl_vertex_data: U,
}

fn vertex(
    mut args: std::str::SplitWhitespace<'_>,
    mut obj_vertex_data: ObjectInfo,
) -> Result<ObjectInfo, Box<dyn Error>> {
    obj_vertex_data.position.push([
        args.next().unwrap().parse()?,
        args.next().unwrap().parse()?,
        args.next().unwrap().parse()?,
    ]);

    Ok(obj_vertex_data)
}

fn vertex_normal(
    mut args: std::str::SplitWhitespace<'_>,
    mut obj_vertex_data: ObjectInfo,
) -> Result<ObjectInfo, Box<dyn Error>> {
    obj_vertex_data.normal.push([
        args.next().unwrap().parse()?,
        args.next().unwrap().parse()?,
        args.next().unwrap().parse()?,
    ]);

    Ok(obj_vertex_data)
}

fn vertex_texture(
    mut args: std::str::SplitWhitespace<'_>,
    mut obj_vertex_data: ObjectInfo,
) -> Result<ObjectInfo, Box<dyn Error>> {
    obj_vertex_data
        .texcoord
        .push([args.next().unwrap().parse()?, args.next().unwrap().parse()?]);

    Ok(obj_vertex_data)
}

fn face(
    mut args: std::str::SplitWhitespace<'_>,
    obj_vertex_data: &ObjectInfo,
    mut gl_vertex_data: VertexDataGroupedThread,
) -> Result<VertexDataGroupedThread, Box<dyn Error>> {
    let first = args.next().unwrap();

    let mut second = args.next().unwrap();
    let mut third;

    for vertex in args {
        third = vertex;
        add_vertex(first, obj_vertex_data, &mut gl_vertex_data)?;
        add_vertex(second, obj_vertex_data, &mut gl_vertex_data)?;
        add_vertex(third, obj_vertex_data, &mut gl_vertex_data)?;
        second = third;
    }

    Ok(gl_vertex_data)
}

fn add_vertex(
    vert: &str,
    obj_vertex_data: &ObjectInfo,
    gl_vertex_data: &mut VertexDataGroupedThread,
) -> Result<(), Box<dyn Error>> {
    let gl_vertex_data = &mut gl_vertex_data.last_group;

    let mut iter = vert.split('/');

    let obj_index = iter.next().unwrap();
    let obj_index: usize = obj_index.parse()?;
    gl_vertex_data
        .position
        .extend_from_slice(&obj_vertex_data.position[obj_index]);

    if let Some(obj_index) = iter.next() {
        let obj_index: usize = obj_index.parse()?;
        gl_vertex_data
            .texcoord
            .extend_from_slice(&obj_vertex_data.texcoord[obj_index]);
    }

    if let Some(obj_index) = iter.next() {
        let obj_index: usize = obj_index.parse()?;
        gl_vertex_data
            .normal
            .extend_from_slice(&obj_vertex_data.normal[obj_index]);
    }

    Ok(())
}

fn group(
    _args: std::str::SplitWhitespace<'_>,
    mut gl_vertex_data: VertexDataGroupedThread,
) -> VertexDataGroupedThread {
    if gl_vertex_data.left_over_completed {
        gl_vertex_data.groups.push(gl_vertex_data.last_group);
    } else {
        gl_vertex_data.left_over = gl_vertex_data.last_group;
        gl_vertex_data.left_over_completed = true;
    }

    gl_vertex_data.last_group = VertexData::new();

    gl_vertex_data
}

#[derive(Debug)]
pub struct VertexDataGrouped {
    pub groups: Vec<VertexData>,
}

impl VertexDataGrouped {
    fn extend(&mut self, groups: VertexDataGroupedThread) {
        if !self.groups.is_empty() {
            self.groups.last_mut().unwrap().extend(groups.left_over);
        }
        self.groups.extend(groups.groups);
        self.groups.push(groups.last_group);
    }
}

#[derive(Debug)]
struct VertexDataGroupedThread {
    left_over: VertexData,
    left_over_completed: bool,
    groups: Vec<VertexData>,
    last_group: VertexData,
}

impl VertexDataGroupedThread {
    fn new() -> Self {
        VertexDataGroupedThread {
            left_over_completed: false,
            left_over: VertexData::new(),
            groups: vec![],
            last_group: VertexData::new(),
        }
    }
}

#[derive(Clone, Debug)]
struct ObjectInfo {
    position: Vec<[f32; 3]>,
    texcoord: Vec<[f32; 2]>,
    normal: Vec<[f32; 3]>,
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
    pub position: Vec<f32>,
    pub texcoord: Vec<f32>,
    pub normal: Vec<f32>,
}

impl VertexData {
    fn extend(&mut self, info: VertexData) {
        self.position.extend(info.position);
        self.normal.extend(info.normal);
        self.texcoord.extend(info.texcoord);
    }

    fn new() -> Self {
        VertexData {
            position: vec![],
            texcoord: vec![],
            normal: vec![],
        }
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
