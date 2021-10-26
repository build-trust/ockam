# no_std example

## Setup

```
rustup target add thumbv7em-none-eabihf --toolchain nightly
brew install qemu
```

## Hello Ockam

```
cargo run --example $(example)
```

```
cargo run --example $(example) --no-default-features --features="alloc, no_std"
```

```
cargo +nightly run --example $(example) --target thumbv7em-none-eabihf --no-default-features --features="qemu"
```

```
cargo +nightly run --example $(example) --target thumbv7em-none-eabihf --no-default-features --features="atsame54"
```

```
cargo +nightly run --example $(example) --target thumbv7em-none-eabihf --no-default-features --features="stm32f4"
```
