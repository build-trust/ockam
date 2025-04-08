/// Higher value means the worker is shutdown earlier
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
pub enum WorkerShutdownPriority {
    /// 1
    Priority1,
    /// 2
    Priority2,
    /// 3
    Priority3,
    /// 4
    #[default]
    Priority4,
    /// 5
    Priority5,
    /// 6
    Priority6,
    /// 7
    Priority7,
}

impl WorkerShutdownPriority {
    /// All possible values in descending order
    pub fn all_descending_order() -> [WorkerShutdownPriority; 7] {
        use WorkerShutdownPriority::*;
        [
            Priority7, Priority6, Priority5, Priority4, Priority3, Priority2, Priority1,
        ]
    }
}
