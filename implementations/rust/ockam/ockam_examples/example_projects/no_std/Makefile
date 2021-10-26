.PHONY: std no_std qemu atsame54 stm32f4

std:
	cargo run --example $(example)

no_std:
	cargo run --example $(example) --no-default-features --features="alloc, no_std"

qemu:
	cargo +nightly run --example $(example) --target thumbv7em-none-eabihf --no-default-features --features="qemu"

atsame54:
	cargo +nightly run --example $(example) --target thumbv7em-none-eabihf --no-default-features --features="atsame54"

stm32f4:
	cargo +nightly run --example $(example) --target thumbv7em-none-eabihf --no-default-features --features="stm32f4"

