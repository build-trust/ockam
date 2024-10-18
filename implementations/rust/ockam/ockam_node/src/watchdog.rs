use std::collections::HashMap;
use std::thread::ThreadId;
use std::time::Duration;
use tokio::runtime::Runtime;

struct ThreadInfo {
    poll_count: u64,
    parked_on_previous_iteration: bool,
    thread_id: ThreadId,
}

/// Detects if there is any thread stuck on a task and write a warning log about it.
/// Pauses the process with SIGSTOP to allow the debugger to attach to the process.
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

    fn watchdog_loop(mut self, tokio_metrics: tokio::runtime::RuntimeMetrics) {
        info!("Starting tokio runtime watchdog");
        const WATCHDOG_INTERVAL: Duration = Duration::from_millis(50);
        let watchdog_interval = std::env::var("OCKAM_WATCHDOG_INTERVAL")
            .map(|s| Duration::from_millis(s.parse().unwrap()))
            .unwrap_or(WATCHDOG_INTERVAL);

        loop {
            // create any missing thread info, threads are always created and never destroyed
            for tokio_worker in 0..tokio_metrics.num_workers() {
                if !self.thread_map.contains_key(&tokio_worker) {
                    if let Some(thread_id) = tokio_metrics.worker_thread_id(tokio_worker) {
                        self.thread_map.insert(
                            tokio_worker,
                            ThreadInfo {
                                poll_count: 0,
                                parked_on_previous_iteration: true,
                                thread_id,
                            },
                        );
                    }
                }
            }

            for (tokio_worker, thread_info) in self.thread_map.iter_mut() {
                // poll_count is used as a proxy to count how many times .await was called
                let poll_count = tokio_metrics.worker_poll_count(*tokio_worker);
                // how many times the thread has been parked and unparked, since it starts as
                // unparked, when odd, the worker is parked
                let is_parked = tokio_metrics.worker_park_unpark_count(*tokio_worker) % 2 == 1;

                // if the worker wasn't parked before, and it's not parked now, and it's still
                // within the same poll count, then it's likely that the worker is stuck
                if !thread_info.parked_on_previous_iteration
                    && !is_parked
                    && thread_info.poll_count == poll_count
                {
                    let ms = watchdog_interval.as_millis();
                    let tid = thread_info.thread_id;
                    let pid = std::process::id();
                    let exe = std::env::current_exe().unwrap();
                    let exe = exe.to_str().unwrap();

                    warn!(
                            "{tid:?} (tokio worker {tokio_worker}) is stuck, and it spent at least {ms} milliseconds without any await.
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
                thread_info.parked_on_previous_iteration = is_parked;
                thread_info.poll_count = poll_count;
            }
            std::thread::sleep(watchdog_interval);
        }
    }
}
