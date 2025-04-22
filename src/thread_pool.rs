use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

type Job = Box<dyn FnOnce() + Send + 'static>;

/// A thread pool for executing tasks concurrently.
///
/// The `ThreadPool` maintains a set of worker threads that can execute tasks
/// submitted to the pool. Tasks are executed in the order they are received.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use keystonelight::ThreadPool;
///
/// let pool = ThreadPool::new(4);
/// pool.execute(|| {
///     println!("Hello from a worker thread!");
/// });
/// ```
///
/// Executing multiple tasks:
///
/// ```
/// use keystonelight::ThreadPool;
/// use std::sync::atomic::{AtomicUsize, Ordering};
/// use std::sync::Arc;
///
/// let pool = ThreadPool::new(4);
/// let counter = Arc::new(AtomicUsize::new(0));
///
/// for _ in 0..10 {
///     let counter = Arc::clone(&counter);
///     pool.execute(move || {
///         counter.fetch_add(1, Ordering::SeqCst);
///     });
/// }
///
/// // Wait for tasks to complete
/// std::thread::sleep(std::time::Duration::from_millis(100));
/// assert_eq!(counter.load(Ordering::SeqCst), 10);
/// ```
///
/// Graceful shutdown:
///
/// ```
/// use keystonelight::ThreadPool;
///
/// let pool = ThreadPool::new(4);
/// pool.execute(|| {
///     println!("Task running");
/// });
///
/// // ThreadPool implements Drop, so it will shut down gracefully
/// // when it goes out of scope
/// ```
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<Sender<Job>>,
}

impl ThreadPool {
    /// Create a new ThreadPool.
    ///
    /// The size is the number of threads in the pool.
    ///
    /// # Panics
    ///
    /// The `new` function will panic if the size is zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use keystonelight::ThreadPool;
    ///
    /// let pool = ThreadPool::new(4);
    /// pool.execute(|| {
    ///     println!("Hello from thread pool!");
    /// });
    /// ```
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = channel();
        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    /// Execute a task in the thread pool.
    ///
    /// The task will be executed by one of the worker threads in the pool.
    /// If the pool has been shut down, the task will be silently dropped.
    ///
    /// # Examples
    ///
    /// ```
    /// use keystonelight::ThreadPool;
    /// use std::sync::atomic::{AtomicUsize, Ordering};
    /// use std::sync::Arc;
    ///
    /// let pool = ThreadPool::new(4);
    /// let counter = Arc::new(AtomicUsize::new(0));
    ///
    /// let counter_clone = Arc::clone(&counter);
    /// pool.execute(move || {
    ///     counter_clone.fetch_add(1, Ordering::SeqCst);
    /// });
    ///
    /// // Wait for task to complete
    /// std::thread::sleep(std::time::Duration::from_millis(100));
    /// assert_eq!(counter.load(Ordering::SeqCst), 1);
    /// ```
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        if let Some(sender) = &self.sender {
            sender.send(job).unwrap();
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        // Drop the sender to signal workers to stop
        drop(self.sender.take());

        // Wait for all workers to finish
        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || {
            loop {
                let job = match receiver.lock().unwrap().recv() {
                    Ok(job) => job,
                    Err(RecvError) => break, // Channel closed, exit thread
                };
                job();
            }
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
}
