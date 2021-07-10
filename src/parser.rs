use std::error::Error;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

const NUM_CORES: usize = 3;

pub fn parse_obj_threaded(obj_file: String) -> Result<VertexDataGrouped, Box<dyn Error>> {
    let (index_str, vertex_str) = extract_vertices_and_indices(obj_file);

    let thread_pool = ThreadPool::new(NUM_CORES);

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
        |keyword, args, _, _, obj_vertex_data, gl_vertex_data| {
            (
                match keyword {
                    "v" => vertex(args, obj_vertex_data).unwrap(),
                    "vn" => vertex_normal(args, obj_vertex_data).unwrap(),
                    "vt" => vertex_texture(args, obj_vertex_data).unwrap(),
                    "g" => obj_vertex_data,
                    "s" => obj_vertex_data,
                    "usemtl" => obj_vertex_data,
                    "mtllib" => obj_vertex_data,
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

fn extract_vertices_and_indices(obj_file: String) -> (Vec<String>, Vec<String>) {
    let (index_str, vertex_str) = obj_file
        .lines()
        .partition::<Vec<_>, _>(|line| line.starts_with('f') || line.starts_with('g'));

    let index_str = chunk_and_combine(index_str);
    let vertex_str = chunk_and_combine(vertex_str);

    (index_str, vertex_str)
}

fn chunk_and_combine(string: Vec<&str>) -> Vec<String> {
    let chunk_size = string.len() / NUM_CORES + 1;
    string
        .into_iter()
        .enumerate()
        .fold(
            (0..NUM_CORES)
                .map(|_| String::with_capacity(chunk_size))
                .collect::<Vec<_>>(),
            |mut arr, (i, line)| {
                arr[i / chunk_size].push('\n');
                arr[i / chunk_size].push_str(line);

                arr
            },
        )
        .into_iter()
        .map(|mut string| {
            string.shrink_to_fit();
            string
        })
        .collect::<Vec<_>>()
}

fn create_parse_thread<T>(
    data: Vec<String>,
    line_handler: T,
    state: State<ObjectInfo, VertexDataGrouped>,
    thread_pool: &ThreadPool,
) -> State<ObjectInfo, VertexDataGrouped>
where
    T: Fn(
            &str,
            std::str::SplitWhitespace,
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
        thread_pool.execute(
            Box::new(move || {
                let mut obj_vertex_data = ObjectInfo {
                    position: vec![],
                    texcoord: vec![],
                    normal: vec![],
                };

                let mut gl_vertex_data = VertexDataGroupedThread::new();

                for line in data[id].lines() {
                    if line.is_empty() || line.starts_with('#') {
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
                        obj_vertex_data: obj_vertex_data,
                        gl_vertex_data: gl_vertex_data,
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

struct Message<T, U> {
    content: State<T, U>,
    id: usize,
}

#[derive(Debug)]
struct State<T, U> {
    obj_vertex_data: T,
    gl_vertex_data: U,
}

fn vertex(
    mut args: std::str::SplitWhitespace,
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
    mut args: std::str::SplitWhitespace,
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
    mut args: std::str::SplitWhitespace,
    mut obj_vertex_data: ObjectInfo,
) -> Result<ObjectInfo, Box<dyn Error>> {
    obj_vertex_data
        .texcoord
        .push([args.next().unwrap().parse()?, args.next().unwrap().parse()?]);

    Ok(obj_vertex_data)
}

fn face(
    mut args: std::str::SplitWhitespace,
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

fn group(
    _args: std::str::SplitWhitespace,
    mut gl_vertex_data: VertexDataGroupedThread,
) -> VertexDataGroupedThread {
    if gl_vertex_data.left_over_completed {
        gl_vertex_data.groups.push(gl_vertex_data.last_group);
        gl_vertex_data.last_group = VertexData::new();
    } else {
        gl_vertex_data.left_over = gl_vertex_data.last_group;
        gl_vertex_data.left_over_completed = true;
        gl_vertex_data.last_group = VertexData::new();
    }

    gl_vertex_data
}

#[derive(Debug)]
pub struct VertexDataGrouped {
    pub groups: Vec<VertexData>,
}

impl VertexDataGrouped {
    fn extend(&mut self, groups: VertexDataGroupedThread) {
        if self.groups.len() != 0 {
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

type Job = Box<dyn FnOnce() + 'static + Send>;

enum ThreadMessage {
    Job(Job),
    Kill,
}

struct ThreadPool {
    senders: Vec<mpsc::Sender<ThreadMessage>>,
    workers: Vec<Worker>,
}

impl ThreadPool {
    fn new(size: usize) -> Self {
        let mut senders = Vec::with_capacity(size);
        let mut workers = Vec::with_capacity(size);
        for _ in 0..size {
            let (tx, rx) = mpsc::channel();
            workers.push(Worker::new(rx));
            senders.push(tx);
        }

        ThreadPool { senders, workers }
    }

    fn execute(&self, work: Job, id: usize) {
        self.senders[id].send(ThreadMessage::Job(work)).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        for sender in &self.senders {
            sender.send(ThreadMessage::Kill).unwrap();
        }

        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

struct Worker {
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(receiver: mpsc::Receiver<ThreadMessage>) -> Self {
        let thread = thread::spawn(move || {
            for work in receiver {
                if let ThreadMessage::Job(work) = work {
                    work();
                } else {
                    break;
                }
            }
        });
        Worker {
            thread: Some(thread),
        }
    }
}
