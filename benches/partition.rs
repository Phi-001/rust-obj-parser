use criterion::{criterion_group, criterion_main, Criterion};
use std::fs;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

extern crate rust_obj_parser;

const NUM_CORES: usize = 4;

fn partion_cases(c: &mut Criterion) {
    let mut group = c.benchmark_group("Partions");
    group.bench_function("partition naive", |b| {
        b.iter(|| {
            let obj_file = fs::read_to_string("al.obj").unwrap();
            let (index_str, vertex_str) = obj_file
                .lines()
                .partition::<Vec<_>, _>(|line| line.starts_with('f'));

            let index_str = chunk_and_combine(index_str);
            let vertex_str = chunk_and_combine(vertex_str);

            (index_str, vertex_str)
        })
    });
    group.bench_function("partition parallel", |b| {
        let thread_pool = ThreadPool::new(NUM_CORES);
        b.iter(|| {
            let obj_file = Arc::new(fs::read_to_string("al.obj").unwrap());
            let len = obj_file.len();
            let chunk_size = len / NUM_CORES + 1;

            let (tx, rx) = mpsc::channel();

            for id in 0..NUM_CORES {
                let tx = tx.clone();
                let obj_file = Arc::clone(&obj_file);
                thread_pool.execute(
                    Box::new(move || {
                        let left_split_index = {
                            let (left, _) = obj_file.split_at(id * chunk_size);
                            left.rfind("\n").unwrap_or(0)
                        };

                        let right_split_index = {
                            if id == NUM_CORES - 1 {
                                len
                            } else {
                                let (_, right) = obj_file.split_at((id + 1) * chunk_size);
                                (id + 1) * chunk_size + right.find("\n").unwrap()
                            }
                        };

                        let chunk = &obj_file[left_split_index..right_split_index];

                        let (index_str, vertex_str) = chunk.lines().fold(
                            (
                                Vec::with_capacity(chunk.len() / 30),
                                Vec::with_capacity(chunk.len() / 30),
                            ),
                            |(mut index, mut vertex), line| {
                                if line.starts_with('f') {
                                    index.push(line.len());
                                } else {
                                    vertex.push(line.len());
                                }

                                (index, vertex)
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

            let mut index_str: Vec<Vec<usize>> = Vec::with_capacity(NUM_CORES);
            let mut vertex_str: Vec<Vec<usize>> = Vec::with_capacity(NUM_CORES);

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
        })
    });
    group.finish();
}

criterion_group!(benches, partion_cases);
criterion_main!(benches);

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

fn extend_fit(data: &mut Vec<Vec<usize>>, extend_data: Vec<usize>, fit_size: usize) {
    let mut last_index = data.last_mut().unwrap();
    let mut space_left = fit_size - last_index.len();

    let mut current_index = 0;

    while space_left <= extend_data.len() - current_index {
        last_index.extend(&extend_data[current_index..current_index + space_left]);
        current_index += space_left;

        data.push(Vec::with_capacity(fit_size));
        last_index = data.last_mut().unwrap();
        space_left = fit_size;
    }

    last_index.extend(&extend_data[current_index..]);
}

struct Message<T> {
    content: T,
    id: usize,
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
