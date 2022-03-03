// This file exists to hack around some limitations cargo workspaces have around
// binary names. The issue is that we need to avoid the `ockam` binary colliding
// with the `ockam` crate.

fn main() {
    ockam_command::run_main()
}
