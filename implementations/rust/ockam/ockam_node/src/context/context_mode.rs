/// Context mode depending on the fact if it's attached to a Worker or a Processor
#[derive(Clone, Copy, Debug)]
pub enum ContextMode {
    /// Without a Worker or a Processor
    Detached,
    /// With a Worker or a Processor
    Attached,
}
