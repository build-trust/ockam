use crate::tokio::runtime::Runtime;
use ockam_core::compat::sync::Arc;

pub struct Metrics {
    rt: Arc<Runtime>,
}

impl Metrics {
    pub(crate) fn new(rt: &Arc<Runtime>) -> Self {
        Self {
            rt: Arc::clone(&rt),
        }
    }

    pub(crate) fn collect_metrics(self: &Arc<Self>) {
        let m = self.rt.metrics();
    }
}
