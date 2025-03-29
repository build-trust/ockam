use crate::tokio::{runtime::Runtime, time};
use core::{
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    time::Duration,
};
use ockam_core::compat::{collections::HashMap, sync::Arc};
use ockam_core::env::get_env;
use opentelemetry::trace::FutureExt;
use std::{fs::OpenOptions, io::Write};
use tokio::runtime::Handle;

pub struct Metrics {
    runtime_handle: Handle,
    router: (Arc<AtomicUsize>, Arc<AtomicUsize>),
}

impl Metrics {
    /// Create a new Metrics collector with access to the runtime
    pub(crate) fn new(
        runtime_handle: Handle,
        router: (Arc<AtomicUsize>, Arc<AtomicUsize>),
    ) -> Arc<Self> {
        Arc::new(Self {
            runtime_handle,
            router,
        })
    }

    pub(crate) fn spawn(self: Arc<Self>, alive: Arc<AtomicBool>) {
        self.runtime_handle
            .spawn(self.run(alive).with_current_context());
    }

    /// Spawned by the Executor to periodically collect metrics
    pub(crate) async fn run(self: Arc<Self>, alive: Arc<AtomicBool>) {
        let path = match get_env::<String>("OCKAM_METRICS_PATH") {
            Ok(Some(path)) => path,
            _ => {
                debug!("Metrics collection disabled, set `OCKAM_METRICS_PATH` to collect metrics");
                return;
            }
        };

        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)
            .expect("failed to open or create metrics collection file");

        file.write_all(b"Worker busy time (% since last poll)\n")
            .expect("failed to write metrics");

        let freq_ms = 100;
        let mut acc = MetricsReport::default();

        loop {
            if !alive.load(Ordering::Relaxed) {
                debug!("Metrics collector shutting down...");
                break;
            }

            let report = self.generate_report(freq_ms, &mut acc);

            file.write_all(format!("{}\n", report.to_csv()).as_bytes())
                .expect("failed to write metrics");
            time::sleep(Duration::from_millis(freq_ms)).await;
        }
    }

    pub(crate) fn generate_report(
        self: &Arc<Self>,
        freq: u64,
        acc: &mut MetricsReport,
    ) -> MetricsReport {
        let m = self.runtime_handle.metrics();

        let tokio_workers = m.num_workers();
        let router_addr_count = self.router.0.load(Ordering::Acquire);
        let router_cluster_count = self.router.1.load(Ordering::Acquire);

        let mut tokio_busy_ms = HashMap::new();
        for wid in 0..tokio_workers {
            // Get the previously accumulated
            let acc_ms = acc.tokio_busy_ms.get(&wid).unwrap_or(&0);
            let raw_ms = m.worker_total_busy_duration(wid).as_millis();

            let diff_ms = raw_ms - acc_ms;
            let percent = diff_ms as f32 / freq as f32;

            tokio_busy_ms.insert(wid, percent as u128);
            acc.tokio_busy_ms.insert(wid, raw_ms);
        }

        MetricsReport {
            tokio_busy_ms,
            router_addr_count,
            router_cluster_count,
        }
    }
}

#[derive(Default)]
#[allow(unused)]
pub struct MetricsReport {
    tokio_busy_ms: HashMap<usize, u128>,
    router_addr_count: usize,
    router_cluster_count: usize,
}

impl MetricsReport {
    /// Generate a line of CSV for this report
    pub fn to_csv(&self) -> String {
        self.tokio_busy_ms
            .iter()
            .map(|(wid, depth)| format!("({}:{}%)", wid, depth))
            .collect::<Vec<String>>()
            .join(",")
    }
}
