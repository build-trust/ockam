pub(crate) use ockam::node::Node;
pub(crate) use ockam::worker::{Worker, WorkerContext};
pub(crate) use ockam::Result;

pub(crate) struct TempPrintWorker {}

pub(crate) type Temperature = u64; // in Kelvin

#[derive(Debug)]
pub(crate) struct TempMeasurement {
    temperature: u32,     // temperature in kelvin
    time_since_boot: u64, // time since boot in seconds
}

impl Worker<TempMeasurement> for TempPrintWorker {
    fn handle(
        &self,
        data: TempMeasurement,
        _context: &WorkerContext<TempMeasurement>,
    ) -> Result<bool> {
        println!("Worker says temp @ {} is {}", data.seconds, data.celsius);
        Ok(true)
    }
}
