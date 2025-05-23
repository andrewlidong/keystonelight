# Keystonelight: A Concurrent Key-Value Store in Rust

## Introduction and Project Goal

Hello everyone. Today I'll be talking about Keystonelight, a Rust-based concurrent key-value database I built during a systems programming course. The goal of this project was to learn how a simple database works under the hood by implementing it from scratch. Keystonelight is essentially a tiny Redis-like server: it stores key-value pairs in memory for speed, but also persists them to disk so nothing is lost on restart. It supports basic operations – get, set, and delete – and clients interact with it over a TCP network connection (so you can connect via a socket and send commands). In building Keystonelight, I had to grapple with real systems programming concepts: managing threads and locks for concurrency, handling file I/O for persistence, designing a simple client-server protocol, and even dealing with Unix signals for process control. In this talk, I'll break down the core features of Keystonelight and share the key technical insights I learned from implementing each component.

## Data Persistence and In-Memory Storage

A fundamental feature of Keystonelight is that it keeps all data in-memory for fast access, while periodically writing updates to a file on disk for persistence. I chose a simple append-only log file approach for the on-disk storage. Every time a client sets or deletes a key, the server appends a record to the log file recording that action. This design is inspired by log-structured storage engines like Bitcask, which write each update sequentially to an append-only log and use memory for indexing.  Writing to disk sequentially like this has two benefits: it's relatively simple to implement, and it's durable – if the server crashes and restarts, it can replay the log file to rebuild the in-memory state of the database. In fact, when Keystonelight starts up, it reads through its log file (if one exists) and reconstructs the hash map of keys in memory. Implementing this from scratch taught me a lot about file I/O in Rust – from basic File operations to ensuring data is flushed to disk properly. I also became aware of the subtle challenges of file locking. For example, I had to consider what would happen if two instances of the server ran at the same time or if a write was interrupted. Properly preventing concurrent file access by multiple processes is tricky (Rust's standard library doesn't have cross-platform file locks out-of-the-box), so I learned why databases often use OS-level file locks or lockfiles. In my case, I kept things simple: only one server process writes the log, and I rely on a PID file (more on that later) to avoid double-starting the server. Even so, thinking through these failure scenarios was an eye-opener – it's easy to corrupt a file with concurrent writes or partial writes, and I gained a healthy respect for techniques like write-ahead logging that ensure atomicity.

Another major aspect of persistence is database compaction – cleaning up the log file. Because we use an append-only log, deletions and overwrites leave old, now-unused entries in the file, which makes the file grow over time. For example, if you set a key "foo" to "bar" and later set "foo" to "baz", the log will have two records for "foo", but only the latest is valid. Similarly, a delete operation just writes a tombstone entry to the log, but the actual data still lingers in the file. To reclaim disk space and prevent the log from growing without bound, Keystonelight performs compaction. Compaction is essentially a garbage-collection for the data file: the server reads the current in-memory state (which represents the latest value for each key) and rewrites the log file from scratch, omitting any deleted keys or old versions. After compaction, the log file contains only live key-value pairs. This process is analogous to what Bitcask does when it merges data files to remove overwritten or deleted entries​.  Implementing compaction taught me about coordinating heavy I/O with the rest of the system. I had to ensure that while compaction is rewriting the file, normal read/write operations are paused or coordinated so they don't interfere. In Keystonelight, I handle this by using a lock (the same lock used for writes, which I'll discuss next) to ensure exclusive access while compacting. It's a simple approach – basically stop the world, compact, then resume – which is acceptable for a small project. From this part of the project, I learned the importance of planning for data maintenance tasks. It's not enough to just write data; a robust system needs a strategy for cleaning up or optimizing storage as well. I also got hands-on experience with file renaming and temporary files to make the compaction as safe as possible (writing the compacted data to a new file, then swapping it in), which is a technique real databases use to avoid losing data if something goes wrong during the process.

## Threading and Concurrency (RwLock)

To make the server handle multiple clients at once, I designed Keystonelight to be multi-threaded. Instead of processing one request at a time, the server can accept a connection and then spawn a worker thread to handle that client's commands while the main thread goes back to listening for new connections. In practice, this means if two clients connect and make requests, they'll be served in parallel by two threads. Rust makes it fairly straightforward to spawn threads, but the challenge is ensuring those threads can safely share access to the key-value store in memory. I used a thread-safe data structure to achieve this: specifically, I wrapped the in-memory hash map in a RwLock (read-write lock) and an Arc (Atomically Reference-Counted pointer). The Arc is necessary to allow multiple threads to own a reference to the same map (since Rust threads require the data to be thread-safe and have static lifetime or be heap-allocated)​.  The RwLock is the concurrency primitive that ensures synchronized access to the shared map. A Rust RwLock allows multiple readers or one writer at a time, which means threads can read from the database in parallel as long as no thread is writing, but a write operation will exclusively lock the map until it's done​.  This was perfect for our use case: many operations might be reads (gets), and we didn't want those to block each other, but if a write (set or delete) comes in, it should have exclusive access to keep data consistent. With the RwLock in place, if three clients do a get concurrently, all three threads can acquire a shared read lock and retrieve values in parallel. If one client issues a set while others are reading, the set will wait until the readers finish, then acquire the write lock, make its modifications (both in memory and to the log file), and release it, after which reads can continue. This strategy – a single global lock for the whole datastore – is coarse-grained but very simple, and it ensured I didn't get race conditions or corrupted memory. I learned that coarse-grained locking can be a good starting point: it's easier to reason about (one lock to rule them all), though it can become a bottleneck if there are many writes. In a more advanced project, one might use more fine-grained locks or lock-free structures to allow even more concurrency, but for Keystonelight an Arc<RwLock<…>> around the hash map was sufficient.

Working with threads and locks in Rust taught me several lessons. First, Rust's ownership and type system guide you to do things properly – for example, I had to use an Arc because you can't just share a &mut HashMap across threads. The compiler forces you to consider thread safety upfront. Second, I became very conscious of lock handling: holding a write lock even a millisecond longer than necessary can stall other threads. In my implementation, I made sure to lock only around the critical sections (like modifying the map and writing to the file) and not, say, around waiting for network I/O. I also had to handle the possibility of a thread panicking while holding the lock – Rust's RwLock is poisoning-aware, meaning if a thread panics, the lock becomes poisoned and subsequent access will error out. In a production server, we'd catch those errors and perhaps reset the state, but in my case I kept things simple and assumed no panics in critical sections. Another interesting challenge was concurrent writes to the log file. Even with the RwLock ensuring only one thread writes at a time, I had to be careful that writes to the file were properly sequenced. By performing the file append while holding the write lock, I effectively serialized all file writes, avoiding interleaving. It's a bit of a brute-force approach (no two writes can happen in parallel, period), but it guaranteed correctness. This gave me an appreciation for how more complex databases might use a separate I/O thread or batching to improve throughput – those would be interesting optimizations to try in the future. But as a developer learning the basics, the big win was seeing that with just a few lines of Rust (Arc::clone, RwLock::write().unwrap(), etc.), I could get safe multi-threaded access to a shared state without data races​.  That was very satisfying, and it demonstrates Rust's strengths in systems programming.

## Networking with a TCP Client-Server Architecture

To make Keystonelight accessible, I implemented a simple TCP-based client-server architecture. The server uses Rust's std::net::TcpListener to bind to a port and listen for incoming client connections. When a client connects (for example, via telnet or a custom client program), the server accepts the connection and wraps it in a TcpStream for reading/writing data. Each client speaks a tiny text-based protocol – basically, the client sends a line like GET key or SET key value, and the server parses that and sends back a response (for get, the value or an error if not found; for set/delete, maybe just an OK). The multi-threading comes into play here: as I mentioned, the server spawns a new thread for each incoming connection (or reuses a pool of threads) so that it can handle many clients concurrently. In code, this looks like: for stream in listener.incoming() { let stream = stream.unwrap(); spawn_thread(handle_connection(stream)); }. Spawning a thread per connection is a straightforward way to achieve concurrency – it means one slow client won't block others, since its handling is on a separate thread​.  I learned that this design is simple but effective for moderate load and was fine for a 5-minute talk demo or a course project. (If we had thousands of clients, we'd need a more sophisticated non-blocking or async approach, but that was beyond our scope.)

One of the challenges I encountered in the networking layer was protocol parsing and error handling. Reading from a TcpStream in Rust gives you raw bytes, and clients might send commands in an unexpected format or even disconnect mid-command. I had to make sure the server handles these gracefully – e.g., using BufReader to read lines and match on the input to decide which operation to perform. I also had to consider when to close the connection. Rust's type system again helped here: the TcpStream is closed automatically when it goes out of scope (and each thread's end of life closes the connection). But I made sure to explicitly handle client quitting or sending a termination command. Another interesting aspect was deciding on a request/response strategy: should the server handle one command per connection or multiple? I opted for handling multiple commands per connection (like a real database server would), which meant the thread for a connection runs in a loop: read a command, execute it, send response, then go back to read the next command. This required careful use of blocking reads so that a thread waiting on a slow client doesn't consume CPU unnecessarily – Rust's blocking I/O was fine for this, though I did consider setting a read timeout to avoid hanging indefinitely. Through implementing the network layer, I learned how low-level some things are when you're not using a framework: I had to manually split on spaces to parse the command and arguments, convert bytes to strings, handle UTF-8, etc. It gave me a newfound respect for libraries and how much they abstract away. Most importantly, it was satisfying to connect to my server with a client and see it responding concurrently. This proved that our thread-safe design worked: two clients could do operations at the same time and each would get correct responses without interfering with each other. The combination of TcpListener for accepting connections and thread-per-connection for concurrency is a classic pattern, and seeing it in action solidified my understanding of how many network servers (like simple web servers or key-value stores) are structured under the hood​.

## Signal Handling and Process Management

Beyond core functionality, I wanted Keystonelight to behave like a well-behaved Unix service. This led me to implement two features: a PID file and signal-triggered compaction. The PID file is a simple mechanism commonly used by daemon processes: when the server starts, it writes its own process ID (PID) to a known file (for example, keystonelight.pid). The file literally just contains the PID number of the running process.  This way, other processes or management scripts (or even a human operator) can easily find out the PID of the server without manually grepping ps. In my case, I used the PID file for two purposes. First, it's a form of lockout – if a new instance of Keystonelight starts, it can check for the existence of that PID file. If the file exists and the PID inside is active, it means another instance is already running, so the new process should refuse to start to avoid conflicts. This prevents the scenario of two servers accidentally writing to the same data file at the same time. Second, the PID file makes it convenient to send signals to the server. For example, to trigger the compaction feature we discussed, one can run a shell command like kill -HUP <pid> (where <pid> is read from the pid file) to send a hangup (SIGHUP) signal to the process. Upon receiving that signal, Keystonelight's signal handler will initiate the log compaction routine. Many Unix services use SIGHUP as a cue to reload configuration or perform maintenance tasks​, so I followed that convention. I also allowed a custom signal (like SIGUSR1) for compaction in case we wanted to reserve SIGHUP for a different meaning, but the principle is the same.

Implementing signal handling in Rust was an enlightening experience. In C, one might use signal() or sigaction to catch signals, but doing this safely in Rust required using an external crate (signal_hook) to register a handler, because Rust restricts what you can do in a signal handler (to prevent messing up memory safety). I learned that signal handlers run asynchronously with respect to your main thread, which means you have to be careful with shared state – you typically can't lock a mutex or allocate memory in a signal handler, for example. The approach I took was to have the signal handler set a flag or write to a pipe that the main thread (or a dedicated signal-listening thread) checks. When the main loop notices the "compaction requested" flag, it then performs the compaction routine in a controlled manner (using the same locking mechanism described earlier to avoid concurrent writes). This way, the heavy lifting of compaction happens in normal thread context, not in the async signal context. It was a bit tricky to get this right; initially I tried triggering compaction directly on the signal, which led to some complications, so I refactored it to the safer approach. The result is that from outside, an operator can send Keystonelight a signal and it will respond by compacting the database on the fly, without shutting down. This was a neat addition – it made the server feel more like a "real" service you might run in production, and it taught me about integrating with operating system features. The PID file and signal handling also reinforced the importance of cleanup: on startup, the server creates the PID file, and on shutdown (or if a fatal error occurs), it should delete that PID file. I had to be mindful of handling abrupt termination too; if the process is killed with SIGKILL, it can't delete its PID file, so a stale PID file might remain. These are the kind of gritty details I got a taste of – in a serious system you might have a separate watchdog or startup script handle clearing stale PID files. In summary, working with signals and PID files taught me how to bridge the gap between my application and the operating system environment. It's not just about writing Rust code that computes things – it's also about playing nicely with process managers, the kernel, and conventions of the OS.

## Conclusion and Lessons Learned

Building Keystonelight was a challenging but rewarding journey through the landscape of systems programming. In about five minutes, I've touched on how I implemented its key features – from an in-memory storage engine with an append-only log and compaction, to a multi-threaded server architecture using locks for thread safety, to the nuts and bolts of network I/O and signal handling. Each component taught me something valuable. Concurrency, for instance, went from an abstract concept to something very concrete: I had to debug real race conditions and lock contention issues, which gave me intuition about why things like read-write locks are useful.  Implementing file persistence and compaction made me appreciate the simplicity and robustness of log-structured designs, but also the need for background maintenance to reclaim space.  Working with the network stack at a low level, I learned how a server actually accepts and manages multiple socket connections, which demystified a lot of what happens inside higher-level frameworks. And handling Unix signals and process IDs connected my program to the operating system control mechanisms, something many high-level application developers never deal with directly.

Perhaps the biggest takeaway is that building a system from scratch forces you to confront the edge cases and hard problems that real-world software deals with. There were moments when I thought, "Why isn't there just a library to do X for me?" – and of course, in production you would use libraries for many of these tasks. But doing it the hard way (at least once) was incredibly educational. It gave me a deeper appreciation for the engineers who create database systems and servers. Even this modest key-value store, while far from production-ready, has so many moving parts that all have to work in concert: memory management, disk I/O, concurrency, consistency, crash recovery, etc. The project also showed me the strengths of Rust for systems programming. Despite dealing with threads, shared memory, and signals – which are traditionally recipes for bugs – Rust's safety guarantees and well-designed std library meant I never had a segfault or memory leak. I certainly had logic bugs while developing (e.g. forgetting to flush the file, or mishandling a lock ordering early on), but once those were fixed, I could run the server under load or kill -9 it and things behaved as expected. For a highly technical audience like this one, I hope this overview provided insight into how one can apply systems-level concepts in a hands-on project. Keystonelight might be "light" in name, but the lessons learned from building it carry a lot of weight for me as a developer.

## Testing Strategy

A key aspect of building a reliable database system is comprehensive testing. For Keystonelight, I implemented a multi-layered testing strategy that covers everything from basic functionality to stress testing under concurrent load.

### Unit and Integration Tests

The foundation of our testing pyramid consists of unit and integration tests. These tests verify core functionality like:
- Basic CRUD operations (Create, Read, Update, Delete)
- Data persistence across restarts
- Log compaction
- Binary data handling
- Error cases and edge conditions

For example, here's a test that verifies data persistence:
```rust
#[test]
fn test_persistence() {
    let temp_dir = tempdir().unwrap();
    let log_file = temp_dir.path().join("keystonelight.log");

    // First instance: write data
    let db = Database::with_log_path(log_file.to_str().unwrap()).unwrap();
    db.set("key1", b"value1").unwrap();
    db.set("key2", b"value2").unwrap();
    drop(db);

    // Second instance: verify data
    let db = Database::with_log_path(log_file.to_str().unwrap()).unwrap();
    assert_eq!(db.get("key1"), Some(b"value1".to_vec()));
    assert_eq!(db.get("key2"), Some(b"value2".to_vec()));
}
```

### Stress Testing

Beyond basic functionality, we need to ensure the system performs reliably under load. The stress tests simulate real-world usage patterns:

1. **Concurrent Operations**: Multiple clients performing random operations simultaneously
2. **Large Data**: Handling values up to 1MB in size
3. **Many Clients**: Testing with up to 20 concurrent clients
4. **Error Injection**: Testing system behavior with invalid inputs
5. **Persistence Under Load**: Verifying data integrity after heavy concurrent writes

Here's an example of a stress test that simulates multiple clients:
```rust
#[test]
fn stress_test_concurrent_operations() {
    let db = Arc::new(Database::with_log_path(log_file).unwrap());
    let num_clients = 5;
    let ops_per_client = 200;

    let mut handles = vec![];
    for client_id in 0..num_clients {
        let db_clone = db.clone();
        let handle = thread::spawn(move || {
            let mut rng = rand::thread_rng();
            for i in 0..ops_per_client {
                let key = format!("key_{}_{}", client_id, i);
                let value = format!("value_{}_{}", client_id, i).into_bytes();

                match rng.gen_range(0..3) {
                    0 => { db_clone.set(&key, &value).unwrap(); }
                    1 => { db_clone.get(&key); }
                    2 => { db_clone.delete(&key).unwrap(); }
                    _ => unreachable!(),
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}
```

### Testing Infrastructure

To support these tests, we've built several testing utilities:
- Temporary directory management using the `tempdir` crate for isolated test runs
- File synchronization helpers to ensure writes are complete
- Cleanup routines to remove test artifacts
- Basic test organization with separate files for different test types (storage, server, stress tests)

The testing infrastructure is designed to be:
- **Reproducible**: Tests run the same way every time
- **Isolated**: Each test has its own clean environment
- **Fast**: Tests run in parallel where possible
- **Comprehensive**: Covering both happy paths and error cases

### Continuous Integration

The project uses GitHub Actions for continuous integration, with a comprehensive CI pipeline that ensures code quality and reliability. The pipeline includes:

1. **Multi-Platform Testing**:
   - Runs on both Ubuntu and macOS
   - Tests with both stable and nightly Rust toolchains
   - Ensures cross-platform compatibility

2. **Code Quality Checks**:
   - Rustfmt for consistent code formatting
   - Clippy for linting and catching common mistakes
   - Build verification on all platforms

3. **Docker Testing**:
   - Runs tests in an isolated Docker environment
   - Ensures consistent test execution across different environments
   - Verifies the Docker configuration works correctly

The CI pipeline runs automatically on:
- Every push to the main branch
- Every pull request targeting the main branch

This ensures that:
- All code changes are properly tested
- Code style and quality standards are maintained
- The project builds and runs correctly across different environments
- Docker-based testing remains functional

The pipeline is configured to fail if any of these checks don't pass, maintaining high code quality standards and preventing regressions.