# Profiling

This directory contains scripts for profiling ockam.
Each script simulates a portal in conjunction with a speed test called `iperf3`.

The scenarios are:
- `portal` - local portal, within one node
- `portal_two_nodes` - two nodes, one inlet and outlet
- `portal_relay` - one node, one inlet and outlet passing through the project relay

Each comes with different variants:
- `baseline` - no profiling, useful for a quick benchmark
- `cpu` - profile CPU usage
- `allocations` - profile memory allocations

## Running the performance tests

To run the performance tests, simply run `tools/profile/<scenario>.<variant>` from the ockam
git root. The script uses the ports 5500 and 8200, and expects an environment without
any other node (otherwise the script might get stuck waiting for a stopped node).

## OS Compatibility
CPU profiling is supported on Linux and MacOS.
Allocation profiling should work on both MacOS and Linux, but MacOS requires the
binary to be signed with an extra capability.
