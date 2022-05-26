use crate::tokio::runtime::Runtime;
use ockam_core::compat::{collections::BTreeMap, sync::Arc};

pub struct Metrics {
    rt: Arc<Runtime>,
}

pub struct MetricsReport {
    tokio_workers: usize,
    tokio_queue_depth: usize,
    io_ready_count: u64,
    worker_queues: BTreeMap<usize, usize>,
}

impl MetricsReport {
    /// Generate a line of CSV for this report
    pub fn to_csv(&self) -> String {
        format!(
            "{},{},{},({})",
            self.tokio_workers,
            self.tokio_queue_depth,
            self.io_ready_count,
            self.worker_queues
                .iter()
                .fold(String::new(), |acc, (wid, queue_depth)| {
                    format!("{},({},{})", acc, wid, queue_depth)
                })
        )
    }
}

impl Metrics {
    pub(crate) fn new(rt: &Arc<Runtime>) -> Self {
        Self {
            rt: Arc::clone(rt),
        }
    }

    pub(crate) fn generate_report(self: &Arc<Self>) -> MetricsReport {
        let m = self.rt.metrics();

        let tokio_workers = m.num_workers();
        let tokio_queue_depth = m.injection_queue_depth();
        let io_ready_count = m.io_driver_ready_count();

        let mut worker_queues = BTreeMap::new();
        for wid in 0..tokio_workers {
            worker_queues.insert(wid, m.worker_local_queue_depth(wid));
        }

        MetricsReport {
            tokio_workers,
            tokio_queue_depth,
            io_ready_count,
            worker_queues,
        }
    }
}
