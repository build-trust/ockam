#[macro_export]
macro_rules! error {
    ($($x:expr),*) => {
        aya_log_ebpf::error!{ $($x,)+ }
    };
}

#[macro_export]
macro_rules! warn {
    ($($x:expr),*) => {
        aya_log_ebpf::warn!{ $($x,)+ }
    }
}

#[macro_export]
macro_rules! debug {
    ($($x:expr),*) => {
        aya_log_ebpf::debug!{ $($x,)+ }
    }
}

#[macro_export]
macro_rules! info {
    ($($x:expr),*) => {
        aya_log_ebpf::info!{ $($x,)+ }
    }
}

#[macro_export]
macro_rules! trace {
    ($($x:expr),*) => {
        aya_log_ebpf::trace!{ $($x,)+ }
    }
}
