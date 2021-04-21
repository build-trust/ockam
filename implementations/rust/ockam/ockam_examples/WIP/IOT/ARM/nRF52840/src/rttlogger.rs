use rtt_target::rprintln;
pub(crate) struct RttLogger;

impl log::Log for RttLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::STATIC_MAX_LEVEL
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        rprintln!(
            "{}:{} -- {}",
            record.level(),
            record.target(),
            record.args()
        );
    }

    fn flush(&self) {}
}
