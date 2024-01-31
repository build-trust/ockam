// This file exists to hack around some limitations cargo workspaces have around
// binary names. The issue is that we need to avoid the `ockam` binary colliding
// with the `ockam` crate.

use ockam_command::util::exitcode;

fn main() {
    if ockam_command::run().is_err() {
        std::process::exit(exitcode::SOFTWARE);
    }
}
