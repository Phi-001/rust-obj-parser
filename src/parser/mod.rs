use std::error::Error;
use std::sync::mpsc;
use std::sync::Arc;

mod thread_pool;

use thread_pool::ThreadPool;

const NUM_CORES: usize = 4;

pub fn parse_obj_threaded(obj_file: String) -> Result<Groups, Box<dyn Error>> {
    let obj_file = Arc::new(obj_file);

    let thread_pool = ThreadPool::new(NUM_CORES);

    let index_vertex = extract_vertices_and_indices(Arc::clone(&obj_file), &thread_pool);

    let vertex_data = parse_vertex(index_vertex.vertex, Arc::clone(&obj_file), &thread_pool);

    let groups = parse_index(
        index_vertex.index,
        vertex_data,
        Arc::clone(&obj_file),
        &thread_pool,
    );

    Ok(groups)
}

fn extract_vertices_and_indices(
    obj_file: Arc<String>,
    thread_pool: &ThreadPool,
) -> IndexVertexInfo {
    let obj_file = Arc::new(obj_file);
    let len = obj_file.len();
    let chunk_size = len / NUM_CORES + 1;

    let (tx, rx) = mpsc::channel();

    thread_pool.execute(|id| {
        let tx = tx.clone();
        let obj_file = Arc::clone(&obj_file);
        Box::new(
            #[inline(never)]
            move || {
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

                let (index, vertex, texcoord, normal, _) = chunk.split_inclusive('\n').fold(
                    (
                        Vec::with_capacity(chunk.len() / 30),
                        Vec::with_capacity(chunk.len() / 30),
                        Vec::with_capacity(chunk.len() / 30),
                        Vec::with_capacity(chunk.len() / 30),
                        left_split_index,
                    ),
                    |(mut index, mut vertex, mut texcoord, mut normal, location), line| {
                        let new_location = location + line.len();

                        if line.starts_with('f') || line.starts_with('g') {
                            index.push((location, new_location));
                        } else {
                            let extend = match line.split_once(' ') {
                                Some(("v", _)) => Some(&mut vertex),
                                Some(("vt", _)) => Some(&mut texcoord),
                                Some(("vn", _)) => Some(&mut normal),
                                _ => None,
                            };

                            if let Some(extend) = extend {
                                extend.push((location, new_location));
                            }
                        }

                        (index, vertex, texcoord, normal, new_location)
                    },
                );

                tx.send(((index, vertex, texcoord, normal), id)).unwrap();
            },
        )
    });

    drop(tx);

    let mut messages = Vec::with_capacity(NUM_CORES);

    let mut index_len = 0;
    let mut position_len = 0;
    let mut texcoord_len = 0;
    let mut normal_len = 0;

    for message in rx {
        let ((index, position, texcoord, normal), _) = &message;
        index_len += index.len();
        position_len += position.len();
        texcoord_len += texcoord.len();
        normal_len += normal.len();
        messages.push(message);
    }

    messages.sort_by(|(_, a_id), (_, b_id)| a_id.cmp(&b_id));

    let mut index = Index::new(index_len);
    let mut vertex = Vertex::new(position_len, texcoord_len, normal_len);

    for message in messages {
        let ((index_extend, position, texcoord, normal), _) = message;

        index.extend_fit(index_extend);
        vertex.extend_fit(position, texcoord, normal);
    }

    IndexVertexInfo { index, vertex }
}

struct IndexVertexInfo {
    index: Index,
    vertex: Vertex,
}

type StartEndPair = (usize, usize);
type Data = Vec<Vec<StartEndPair>>;

struct Index {
    data: Data,
    size: usize,
}

impl Index {
    fn new(length: usize) -> Self {
        let mut index = Index {
            data: Vec::with_capacity(NUM_CORES),
            size: length,
        };

        index.data.push(Vec::with_capacity(length / NUM_CORES + 1));

        index
    }

    fn extend_fit(&mut self, extend_data: Vec<StartEndPair>) {
        extend_fit(&mut self.data, extend_data, self.size / NUM_CORES + 1);
    }
}

struct Vertex {
    position: Data,
    texcoord: Data,
    normal: Data,
    position_size: usize,
    texcoord_size: usize,
    normal_size: usize,
}

impl Vertex {
    fn new(position_size: usize, texcoord_size: usize, normal_size: usize) -> Self {
        let mut vertex = Vertex {
            position: Vec::with_capacity(NUM_CORES),
            texcoord: Vec::with_capacity(NUM_CORES),
            normal: Vec::with_capacity(NUM_CORES),
            position_size,
            texcoord_size,
            normal_size,
        };

        vertex
            .position
            .push(Vec::with_capacity(position_size / NUM_CORES + 1));
        vertex
            .texcoord
            .push(Vec::with_capacity(texcoord_size / NUM_CORES + 1));
        vertex
            .normal
            .push(Vec::with_capacity(normal_size / NUM_CORES + 1));

        vertex
    }

    fn extend_fit(
        &mut self,
        extend_position: Vec<StartEndPair>,
        extend_texcoord: Vec<StartEndPair>,
        extend_normal: Vec<StartEndPair>,
    ) {
        extend_fit(
            &mut self.position,
            extend_position,
            self.position_size / NUM_CORES + 1,
        );
        extend_fit(
            &mut self.texcoord,
            extend_texcoord,
            self.texcoord_size / NUM_CORES + 1,
        );
        extend_fit(
            &mut self.normal,
            extend_normal,
            self.normal_size / NUM_CORES + 1,
        );
    }
}

fn extend_fit(data: &mut Data, extend_data: Vec<(usize, usize)>, fit_size: usize) {
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

fn parse_vertex(vertex: Vertex, obj_file: Arc<String>, thread_pool: &ThreadPool) -> VertexData {
    let Vertex {
        position_size,
        texcoord_size,
        normal_size,
        position,
        normal,
        texcoord,
        ..
    } = vertex;

    let mut vertex_data = VertexData::with_capacity(position_size, texcoord_size, normal_size);

    let position_ptr = FloatPtr(vertex_data.position.as_mut_ptr());
    let texcoord_ptr = FloatPtr(vertex_data.texcoord.as_mut_ptr());
    let normal_ptr = FloatPtr(vertex_data.normal.as_mut_ptr());

    let (tx, rx) = mpsc::channel();

    let position = Arc::new(position);
    let texcoord = Arc::new(texcoord);
    let normal = Arc::new(normal);

    thread_pool.execute(|id| {
        let obj_file = Arc::clone(&obj_file);
        let tx = tx.clone();
        let position_ptr = position_ptr.clone();
        let texcoord_ptr = texcoord_ptr.clone();
        let normal_ptr = normal_ptr.clone();
        let position = Arc::clone(&position);
        let texcoord = Arc::clone(&texcoord);
        let normal = Arc::clone(&normal);
        Box::new(
            #[inline(never)]
            move || {
                let FloatPtr(mut position_ptr) = position_ptr;
                let FloatPtr(mut texcoord_ptr) = texcoord_ptr;
                let FloatPtr(mut normal_ptr) = normal_ptr;

                unsafe {
                    position_ptr = position_ptr.add(3 * id * (position_size / NUM_CORES + 1));
                    texcoord_ptr = texcoord_ptr.add(2 * id * (texcoord_size / NUM_CORES + 1));
                    normal_ptr = normal_ptr.add(3 * id * (normal_size / NUM_CORES + 1));
                }

                for (data, mut ptr) in [
                    (position, position_ptr),
                    (normal, normal_ptr),
                    (texcoord, texcoord_ptr),
                ] {
                    if data.len() != NUM_CORES {
                        continue;
                    }

                    for &(start, end) in &data[id] {
                        let line = &obj_file[start..end].trim();

                        if line.is_empty() {
                            continue;
                        }

                        let mut parts = line.split_whitespace();

                        parts.next().unwrap();

                        for num in parts {
                            let num = num.parse().unwrap();

                            unsafe {
                                ptr.write(num);
                                ptr = ptr.add(1);
                            }
                        }
                    }
                }

                tx.send(()).unwrap();
            },
        )
    });

    drop(tx);

    for _ in rx {}

    unsafe {
        vertex_data.position.set_len(position_size * 3);
        vertex_data.texcoord.set_len(texcoord_size * 2);
        vertex_data.normal.set_len(normal_size * 3);
    }

    vertex_data
}

#[derive(Clone)]
struct FloatPtr(*mut f32);

// No idea if this is actually safe or not
// Hopefully so
unsafe impl Send for FloatPtr {}
unsafe impl Sync for FloatPtr {}

fn parse_index(
    index: Index,
    vertex_data: VertexData,
    obj_file: Arc<String>,
    thread_pool: &ThreadPool,
) -> Groups {
    let (tx, rx) = mpsc::channel();

    let index = Arc::new(index.data);
    let vertex_data = Arc::new(vertex_data);

    thread_pool.execute(|id| {
        let index = Arc::clone(&index);
        let obj_file = Arc::clone(&obj_file);
        let tx = tx.clone();
        let vertex_data = Arc::clone(&vertex_data);
        Box::new(
            #[inline(never)]
            move || {
                let mut groups = vec![VertexData::new()];

                for &(start, end) in &index[id] {
                    let line = &obj_file[start..end].trim();

                    if line.is_empty() {
                        continue;
                    }

                    let mut parts = line.split_whitespace();

                    let keyword = parts.next().unwrap();

                    match keyword {
                        "g" => {
                            groups.push(VertexData::new());
                        }
                        "f" => {
                            let group = groups.last_mut().unwrap();
                            let first = parts.next().unwrap();

                            let mut second = parts.next().unwrap();
                            let mut third;

                            for vertex in parts {
                                third = vertex;
                                add_vertex(first, group, &*vertex_data);
                                add_vertex(second, group, &*vertex_data);
                                add_vertex(third, group, &*vertex_data);
                                second = third;
                            }
                        }
                        _ => {}
                    }
                }

                tx.send((groups, id)).unwrap();
            },
        )
    });

    drop(tx);

    let mut messages = vec![];

    for message in rx {
        messages.push(message);
    }

    messages.sort_by(|(_, id_a), (_, id_b)| id_a.cmp(&id_b));

    let mut groups: Vec<Group> = vec![];

    for (group, _) in messages {
        let mut iter = group.into_iter();
        let first = iter.next().unwrap();
        if let Some(last) = groups.last_mut() {
            last.extend(first);
        }
        groups.extend(iter);
    }

    groups
}

fn add_vertex(vert: &str, dst: &mut VertexData, src: &VertexData) {
    let mut iter = vert.split('/');

    let obj_index = iter.next().unwrap();
    let obj_index = obj_index.parse::<usize>().unwrap() - 1;
    dst.position
        .extend_from_slice(&src.position[obj_index * 3..obj_index * 3 + 3]);

    if let Some(obj_index) = iter.next() {
        let obj_index = obj_index.parse::<usize>().unwrap() - 1;
        dst.texcoord
            .extend_from_slice(&src.texcoord[obj_index * 2..obj_index * 2 + 2]);
    }

    if let Some(obj_index) = iter.next() {
        let obj_index = obj_index.parse::<usize>().unwrap() - 1;
        dst.normal
            .extend_from_slice(&src.normal[obj_index * 3..obj_index * 3 + 3]);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VertexData {
    pub position: Vec<f32>,
    pub texcoord: Vec<f32>,
    pub normal: Vec<f32>,
}

impl VertexData {
    fn new() -> Self {
        VertexData {
            position: Vec::with_capacity(200),
            texcoord: Vec::with_capacity(0),
            normal: Vec::with_capacity(0),
        }
    }

    fn with_capacity(position_size: usize, texcoord_size: usize, normal_size: usize) -> Self {
        VertexData {
            position: Vec::with_capacity(position_size * 3),
            texcoord: Vec::with_capacity(texcoord_size * 2),
            normal: Vec::with_capacity(normal_size * 3),
        }
    }

    fn extend(&mut self, data: VertexData) {
        self.position.extend(data.position);
        self.texcoord.extend(data.texcoord);
        self.normal.extend(data.normal);
    }
}

type Group = VertexData;
type Groups = Vec<Group>;
