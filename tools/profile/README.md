# Profiling

This directory contains tools for profiling ockam.

Two scenarios for performance profiling:
- `portal.perf` - local portal, within one node
- `portal_two_nodes.perf` - two nodes, one inlet and outlet
- `relay_port.perf` - one node, one inlet and outlet passing through a relay

And one scenario for heap profiling:
- `portal.valgrind.dhat` - local portal, within one node

## Running the performance tests

To run the performance tests, simply run `tools/profile/SCRIPT` from the ockam
git root.

## OS Compatibility
The performance scripts are currently compatible only with Linux since they use `perf`.
On MacOS, a similar approach should be doable with `dtrace`, but is not yet implemented.

Heap profiling with valgrind is compatible with both Linux and MacOS.
