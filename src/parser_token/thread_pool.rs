use std::sync::mpsc;
use std::thread;

type Job = Box<dyn FnOnce() + 'static + Send>;

pub struct ThreadPool {
    senders: Option<Vec<mpsc::Sender<Job>>>,
    workers: Vec<Worker>,
    pub size: usize,
}

impl ThreadPool {
    pub fn new(size: usize) -> Self {
        let mut senders = Vec::with_capacity(size);
        let mut workers = Vec::with_capacity(size);

        for _ in 0..size {
            let (tx, rx) = mpsc::channel();
            workers.push(Worker::new(rx));
            senders.push(tx);
        }

        ThreadPool {
            senders: Some(senders),
            workers,
            size,
        }
    }

    fn execute_id(&self, work: Job, id: usize) {
        if let Some(senders) = &self.senders {
            senders[id].send(work).unwrap();
        }
    }

    pub fn execute<T>(&self, work: T)
    where
        T: Fn(usize) -> Job,
    {
        for id in 0..self.size {
            self.execute_id(work(id), id);
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        if let Some(senders) = self.senders.take() {
            for _ in senders {}
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
    fn new(receiver: mpsc::Receiver<Job>) -> Self {
        let thread = thread::spawn(move || {
            for work in receiver {
                work();
            }
        });
        Worker {
            thread: Some(thread),
        }
    }
}
