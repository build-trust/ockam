// This file exists to hack around some limitations cargo workspaces have around
// binary names. The issue is that we need to avoid the `ockam` binary colliding
// with the `ockam` crate.
use tikv_jemallocator::Jemalloc;
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

use ockam_command::util::exitcode;

fn main() {
    if let Err(e) = ockam_command::entry_point::run() {
        // initialization errors are displayed here
        eprintln!("{:?}", e);
        std::process::exit(exitcode::SOFTWARE);
    }
}
