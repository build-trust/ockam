# no_std example

This example project shows how to create a node running in a `no_std` environment.

## Setup

```
rustup target add thumbv7em-none-eabihf --toolchain nightly
brew install qemu
```

## Hello Ockam

```
cargo run --example hello
```

```
cargo run --example hello --no-default-features --features="alloc, no_std"
```

```
cargo +nightly run --example hello --target thumbv7em-none-eabihf --no-default-features --features="qemu"
```

```
cargo +nightly run --example hello --target thumbv7em-none-eabihf --no-default-features --features="atsame54"
```

```
cargo +nightly run --example hello --target thumbv7em-none-eabihf --no-default-features --features="stm32f4"
```
