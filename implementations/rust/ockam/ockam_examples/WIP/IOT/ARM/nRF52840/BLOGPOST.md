# How to debug nrf52840

## Debugging with a Hardware

Tests on hardware can be done trifold (marked with `x` which were used):

1. [x] IDE + gdb server via `probe-rs`
2. [x] logging - `cargo-embed` or `cargo-flash` with `rtt` enabled and an `rtt` logger on target
3. [ ] pin toggle & oszilloscope

The first is the goto method as with regular rust programs, but at times it changes behaviour, i.e. breakpoints in irqs.
Logging is another ubquitorious method of extracting information, yet the rate is very limited by rtt and it comes with
some latency.
Pin toggling from key points - especially IRQs - is another, though very limited in information flow, tool iff an oscilloscope is available.

## Debugging without a Hardware & HAL

Debugging without hardware and HAL is generally possible, yet it needs a clear cut between business logic and anything HAL related.
This allows compilation for a host target.
As such mock types for all kinds of futures are required and some aspects cannot be modeled such as runtime behaviour or measuring power consumption or accurate latency estimations.
