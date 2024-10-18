use once_cell::sync::Lazy;
use std::sync::Mutex;
use tokio::runtime::Runtime;

pub(crate) static RUNTIME: Lazy<Mutex<Option<Runtime>>> =
    Lazy::new(|| Mutex::new(Some(Runtime::new().unwrap())));

/// Return the Runtime singleton
/// This function can only be accessed once
pub fn take() -> Runtime {
    RUNTIME
        .lock()
        .unwrap()
        .take()
        .expect("Runtime was consumed")
}

#[cfg(feature = "watchdog")]
pub(crate) mod watchgod {
    use std::collections::HashMap;
    use std::thread::ThreadId;
    use std::time::Duration;
    use tokio::runtime::Runtime;

    struct ThreadInfo {
        poll_count: u64,
        was_parked: bool,
        thread_id: ThreadId,
    }

    /// Detects if there is any thread stuck on a task and write a warning log about it.
    /// Print its stacktrace if possible.
    pub(crate) struct TokioRuntimeWatchdog {
        thread_map: HashMap<usize, ThreadInfo>,
    }

    impl TokioRuntimeWatchdog {
        pub(crate) fn new() -> Self {
            Self {
                thread_map: HashMap::new(),
            }
        }

        pub fn start_watchdog_loop(self, runtime: &Runtime) {
            let metrics = runtime.metrics();
            // to avoid spawning the watchdog within tokio runtime
            std::thread::spawn(move || {
                self.watchdog_loop(metrics);
            });
        }

        fn watchdog_loop(mut self, metrics: tokio::runtime::RuntimeMetrics) {
            info!("Starting tokio runtime watchdog");
            const WATCHDOG_INTERVAL: Duration = Duration::from_millis(50);
            let watchdog_interval = std::env::var("OCKAM_WATCHDOG_INTERVAL")
                .map(|s| Duration::from_millis(s.parse().unwrap()))
                .unwrap_or(WATCHDOG_INTERVAL);

            loop {
                // create any missing thread info
                for worker in 0..metrics.num_workers() {
                    if !self.thread_map.contains_key(&worker) {
                        if let Some(thread_id) = metrics.worker_thread_id(worker) {
                            self.thread_map.insert(
                                worker,
                                ThreadInfo {
                                    poll_count: 0,
                                    was_parked: true,
                                    thread_id,
                                },
                            );
                        }
                    }
                }

                for (worker, thread_info) in self.thread_map.iter_mut() {
                    let poll_count = metrics.worker_poll_count(*worker);
                    // when odd, the worker is parked
                    let is_parked = metrics.worker_park_unpark_count(*worker) % 2 == 1;

                    // if the worker wasn't parked before, and it's not parked now, and it's still
                    // within the same poll count, then it's likely that the worker is stuck
                    if !thread_info.was_parked && !is_parked && thread_info.poll_count == poll_count
                    {
                        let ms = watchdog_interval.as_millis();
                        let tid = thread_info.thread_id;
                        let pid = std::process::id();
                        let exe = std::env::current_exe().unwrap();
                        let exe = exe.to_str().unwrap();

                        warn!(
                            "{tid:?} (tokio worker {worker}) is stuck, and it spent at least {ms} milliseconds without any await.
                            In order to debug the issue the process was paused with SIGSTOP.
                            False positives are unlikely but possible.
                            You can attach the rust debugger with:
                                rust-lldb {exe} --attach-pid {pid}
                            and then select the thread with:
                                thread select THREAD_ID
                            and print the backtrace with:
                                bt
                            "
                        );

                        // Send SIGSTOP to self to pause the process in this precise moment in time
                        // so the debugger can attach to the process, and inspect the threads.
                        // SIGSTOP is sent twice to work around the debugger that sometimes
                        // issues a SIGCONT when attaching to the process.
                        for _n in 0..2 {
                            let _ = nix::sys::signal::raise(nix::sys::signal::Signal::SIGSTOP);
                        }
                    }
                    thread_info.was_parked = is_parked;
                    thread_info.poll_count = poll_count;
                }
                std::thread::sleep(watchdog_interval);
            }
        }
    }
}
