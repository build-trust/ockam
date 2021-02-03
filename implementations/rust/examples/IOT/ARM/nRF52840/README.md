# nrf52840-worker-example

A demo project using the ockam worker API in the final stage.

Use `cargo embed` and `cargo flash` for flashing and `probe-run` as helper to to run the program from `vs-code` (optional).

The board / chip in use is the nordic NRF52840-DK development board.
The BLE functionality with this device is deliberately not used for this purpose.

The chip is ARM Cortex-M4 based with various peripherals, i.e. real time clock, timers and the usual set, besides a high speed speed comparator and peripheral programming which allows for low latency connection.

Flash Memory range is from `0x0000_0000..0x0010_0000`
(this overrides the softdevice as well), SRAM accessible in the range `0x2000_0000..0x2004_0000`.

Note that these ranges are not compatible with the nrf52840-_dongle_, in both regions the start must be sufficiently large to host the bootloader state in SRAM as well as the so called `softdevice` BLE stack at start of the Flash.

Heap starts after the symbol `__sheap` as defined by [link.x](https://github.com/rust-embedded/cortex-m-rt/blob/master/link.x.in).

Buttons and LEDs are active low as described in [nrf52840_dk](https://infocenter.nordicsemi.com/index.jsp?topic=%2Fug_nrf52840_dk%2FUG%2Fdk%2Fhw_buttons_leds.html) guide.

To avoid hot-spinning, `direct_executor` is used instead of `executor` in conjunction with bool token that is passed through `Flag`.

## Retrospective

It would have been significantly easier to just go with `rtic` from the get go, but that would
a) not have demonstrate much and b) would not be compatible with the execution model of the current
async flow.

## API improvements for no_std

Assumptions:

* bare metal, no framework

or

* library usage

### crate: ockam_node_attribute

* use `proc_macro2` to be able to unit test it
* allow target specific wake + sleep functions

### crate: ockam_node_no_std

* use `direct_executor` instead to allow `wake`+`sleep` and avoid hot spinning

### crate: ockam

* remove all `Arc<Mutex<_>>` references and replace by `RefCell` where necessary
* figure out if multi state devices, such that multiple nodes are relevant
  * use unit structs and ownership to encode exclusivity of node states
* avoid `dyn` where possible and prefer generics with trait bounds
* avoid `hashmap`s, expect the user to provide an upper bound of workers to be registered

## Comment: rtic

Currently the more radical concept which is employed by RTIC to provide true preemptive scheduling
on ARM Cortex M CPUs via the NVIC priorities seems to have most traction. The current design would
not fit with that well, the program code structure is very different.
