#![allow(dead_code)]
use std::error::Error;
use std::sync::mpsc;
use std::sync::Arc;

mod thread_pool;
mod tokenizer;

use tokenizer::ObjFile;

use thread_pool::ThreadPool;

const NUM_CORES: usize = 4;

pub fn parse_obj_threaded(obj_file: String) -> Result<Groups, Box<dyn Error>> {
    let obj_file = Arc::new(obj_file);

    let thread_pool = ThreadPool::new(NUM_CORES);

    let obj_file = creat_tokenized_obj_file(Arc::clone(&obj_file), &thread_pool);

    let groups = parse_tokens(obj_file, &thread_pool);

    Ok(groups)
}

fn creat_tokenized_obj_file(obj_file: Arc<String>, thread_pool: &ThreadPool) -> ObjFile {
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

                tx.send((ObjFile::new(&mut chunk.bytes().peekable()), id))
                    .unwrap();
            },
        )
    });

    drop(tx);

    let mut messages = Vec::with_capacity(NUM_CORES);

    for message in rx {
        messages.push(message);
    }

    messages.sort_by(|(_, a_id), (_, b_id)| a_id.cmp(&b_id));

    let mut obj_file = ObjFile {
        position: vec![],
        normal: vec![],
        texcoord: vec![],
        index: vec![],
    };

    for (message, _) in messages {
        obj_file.position.extend(message.position);
        obj_file.texcoord.extend(message.texcoord);
        obj_file.normal.extend(message.normal);
        obj_file.index.extend(message.index);
    }

    println!("{}", obj_file.position.len());
    println!("{}", obj_file.index.len());

    obj_file
}

fn parse_tokens(obj_file: ObjFile, thread_pool: &ThreadPool) -> Groups {
    let (tx, rx) = mpsc::channel();

    let obj_file = Arc::new(obj_file);

    thread_pool.execute(|id| {
        let obj_file = Arc::clone(&obj_file);
        let tx = tx.clone();
        Box::new(
            #[inline(never)]
            move || {
                let mut groups = vec![VertexData::new()];

                let chunk_size = obj_file.index.len() / NUM_CORES + 1;

                let start = id * chunk_size;

                let end = if id == NUM_CORES - 1 {
                    obj_file.index.len()
                } else {
                    start + chunk_size
                };

                let chunk = &obj_file.index[start..end];

                for index in chunk {
                    match index {
                        tokenizer::IndexGroup::Group(_) => groups.push(VertexData::new()),
                        tokenizer::IndexGroup::Index(face) => {
                            for index in &face.indices {
                                let group = groups.last_mut().unwrap();

                                if let Some(position_index) = index.position {
                                    group.position.extend_from_slice(
                                        &obj_file.position[position_index - 1].position,
                                    );
                                }

                                if let Some(texcoord_index) = index.texcoord {
                                    group.texcoord.extend_from_slice(
                                        &obj_file.texcoord[texcoord_index - 1].texcoord,
                                    );
                                }

                                if let Some(normal_index) = index.normal {
                                    group.normal.extend_from_slice(
                                        &obj_file.normal[normal_index - 1].normal,
                                    );
                                }
                            }
                        }
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

    fn extend(&mut self, data: VertexData) {
        self.position.extend(data.position);
        self.texcoord.extend(data.texcoord);
        self.normal.extend(data.normal);
    }
}

type Group = VertexData;
type Groups = Vec<Group>;
