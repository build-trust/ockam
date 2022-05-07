# How to run Ockam on RISC-V Linux

In this hands-on guide, we'll show how to cross compile a Rust example of Ockam for RISC-V Linux systems.
We'll also see how to test RISC-V Linux programs using qemu.

Before we dive in, let's start with a ~2 minute demo of how to run Ockam's end-to-end encryption example on Microchip's PolarFire SoC Icicle kit.

https://user-images.githubusercontent.com/159583/140436789-09d4236d-83cd-4c45-964f-3be18b554a3f.mp4

<div align="center"><i>Please unmute to listen to the explanation of the demo</i></div>

## What is RISC-V?

RISC-V is a free, open CPU instruction set architecture (ISA) that is revolutionizing hardware. The open source nature of the ISA has allowed global innovation and creativity in much the same way Open Source revolutionized software. Since
its first publication from UC Berkeley in 2011, adoption of RISC-V has seen incredible growth. Organizations like RISC-V International collaborate with dozens of companies, academic institutions, and passionate individuals to propel the
ecosystem forward.

Open architectures such as RISC-V provide transparency down to the deepest levels of a system. For systems that require strong security and safety guarantees, an open ISA provides the foundation of a completely understood supply chain.

RISC-V is also extensible: additional instructions can be added to perform new operations. This ability is being used to add many features to the ISA, including cryptographic extensions. These extensions enable the development of specialized processors such as Secure Enclaves and Trusted Execution Environments.

## About the RISC-V ISA

RISC-V cores come in many shapes and sizes, because the ISA is so flexible and extensible.

There are 32-bit, 64-bit and even 128-bit versions of the core integer instructions.

- rv32 is the 32-bit configuration suitable for microcontrollers and other small, low power systems.
- rv64 is the 64-bit general purpose, server class configuration. RISC-V Linux projects target the RV64.
- rv128 is a mostly theoretical 128-bit configuration.

RISC-V cores are also configurable using what are called instruction set variants. Variants add additional instructions to the core ISA. Several variants have been officially standardized, and more are on the way. Variants are referenced
by a single letter code such as I, M and C.

The most important RISC-V variants are:
- **I** - The base integer instructions. All RISC-V cores implement this.
- **M** - Multiplication instructions. Most cores implement this, but some very small microcontrollers do not.
- **A** - Atomic instructions. These instructions enable multi-core systems to have consistent views of memory.
- **F** - Floating point instructions. Often omitted on microcontroller configurations.
- **D** - Double floating point instructions. Often omitted on microcontroller configurations.
- **C** - Compressed instructions. Similar to ARM's Thumb instructions. Reduces code size. Sometimes omitted.
- **G** - Short hand for the set of **IMAFD** variants.

RISC-V cores are referred to by their bit-width and set of supported variants. For example, a common class of microcontroller configurations is `rv32imc` which is a 32-bit processor with a multiplier and compressed instructions.

The RISC-V configuration targeted by Linux distributions is typically `rv64gc`, although it is possible to run Linux on other configurations.

## Rust on RISC-V

Rust supports several RISC-V targets. Rust targets are split into [several tiers](https://doc.rust-lang.org/nightly/rustc/platform-support.html),
which have different guarantees with respect to support and stability.

- `riscv64-unknown-linux-gnu` is a "Tier 2 With Host Tools" target that has support for `std` running on Linux.
- `riscv64gc-unknown-none-elf` and `riscv64imac-unknown-none-elf` are Tier 2 targets that can be used in bare metal projects.
- `riscv32i`, `riscv32imc` and `riscv32imac` `-unknown-none-elf` are the 32-bit Tier 2 targets that can be used in bare metal projects.
- `riscv32imc-esp-espidf` is a Tier 3 target that supports ESP RISC-V chips like the esp32-c3.

Rust programs targeting RISC-V can be built with or without the `std` library. Building `no_std` applications is a complex topic, and requires details about the underlying hardware being targeted. Additionally, an allocator is often needed if the application requires a heap.

This guide focuses on `std` projects for RISC-V running Linux. Ockam support for `no_std` is an ongoing effort with initial support for several ARM boards. In the future, Ockam `no_std` support will be extended to the RISC-V ecosystem.

Cross compiling for a different processor requires the presence of a toolchain and libraries for that target. There are a variety of ways to cross compile to RISC-V:

- Use [cargo cross](https://github.com/rust-embedded/cross). This is the fastest and easiest way to get started.
- Install cross compiler packages from your OS distribution. Debian for example, has many riscv64 tools.
- Build and install the official [riscv-gnu-toolchain](https://github.com/riscv-collab/riscv-gnu-toolchain).

### Rust on RISC-V with cargo cross

First, install cross:

```bash
cargo install cross
```

Create a new Rust project:

```bash
cargo new --bin ockam_rv
```

Add the Ockam dependency to the project's `Cargo.toml` dependencies:

```toml
[dependencies]
ockam = "0"
```

Now let's turn `main.rs` into an Ockam Node:

```rust
use ockam::*;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    println!("Hello, Ockam!");
    ctx.stop().await
}
```

`cross` takes the same arguments as `cargo`. To build the project, run:

```bash
cross build --target riscv64gc-unknown-linux-gnu
```

This will generate a RISC-V binary in the `target/riscv64gc-unknown-linux-gnu/debug/` directory!

You can also run the project in an emulated environment with `cross`:

```bash
cross run --target riscv64gc-unknown-linux-gnu
```

You should see output similar to:

```
Finished dev [unoptimized + debuginfo] target(s) in 0.04s
Running `/linux-runner riscv64 /target/riscv64gc-unknown-linux-gnu/debug/ockam_rv`
2021-11-03T19:52:01.495439Z  INFO ockam_node::node: Initializing ockam node
Hello, Ockam!
2021-11-03T19:52:01.586869Z  INFO ockam_node::context: Shutting down all workers
```

In most cases, `cross` is sufficient for cross compiling to RISC-V Linux. However, it does require Docker or Podman.

### Hello Ockam

Let's create an encrypted secure channel between Alice and Bob. When a message is sent through this channel it will be encrypted when it enters the channel and decrypted just before it exits the channel.

For the purpose of our example, we'll create a local channel within one program. In our [other examples](https://github.com/build-trust/ockam/tree/develop/documentation/guides/rust#readme), you'll see that it's just as easy to create end-to-end protected channels over multi-hop, multi-protocol transport routes:

Replace the contents of `src/main.rs` with the following code:

```rust

use ockam::{route, Context, Identity, Result, TrustEveryonePolicy, Vault};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Create a Vault to safely store secret keys for Alice and Bob.
    let vault = Vault::create();

    // Create an Identity to represent Bob.
    let mut bob = Identity::create(&ctx, &vault).await?;

    // Create a secure channel listener for Bob that will wait for requests to
    // initiate an Authenticated Key Exchange.
    bob.create_secure_channel_listener("bob", TrustEveryonePolicy).await?;

    // Create an entity to represent Alice.
    let mut alice = Identity::create(&ctx, &vault).await?;

    // As Alice, connect to Bob's secure channel listener and perform an
    // Authenticated Key Exchange to establish an encrypted secure channel with Bob.
    let channel = alice.create_secure_channel("bob", TrustEveryonePolicy).await?;

    // Send a message, ** THROUGH ** the secure channel,
    // to the "app" worker on the other side.
    //
    // This message will automatically get encrypted when it enters the channel
    // and decrypted just before it exits the channel.
    ctx.send(route![channel, "app"], "Hello Ockam!".to_string()).await?;

    // Wait to receive a message for the "app" worker and print it.
    let message = ctx.receive::<String>().await?;
    println!("App Received: {}", message); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
```

A lot happened when you ran this small example. It created a secure vault, spawned workers to represent entities, established a mutually authenticated channel and then routed a message through that channel. This involved running cryptographic protocols for generating keys, authenticating as an entity, performing an authenticated key exchange and exchanging messages with authenticated encryption.

### Rust on RISC-V with OS toolchains

The process for installing cross-compiling tools differs between OSes and distributions. The primary requirement for
building Rust executables for RISC-V is the presence of a RISC-V linker, such as GCC.

- For Debian: `apt-get install gcc-riscv64-linux-gnu`
- For MacOS: `brew tap riscv/riscv && brew install --cc=gcc-10 riscv-tools`

In your `ockam_rv` project, create a `.cargo/config.toml` file. In this file, we specify that we're builing for RISC-V
and also configure the location of the linker.


```toml
[build]
target = "riscv64gc-unknown-linux-gnu"

[target.riscv64gc-unknown-linux-gnu]
linker = "/path/to/your/riscv64-unknown-linux-gnu-gcc"
```

Now when you run `cargo build`, your local cross-compiling toolchain will be used to link the program, which is available in `target/riscv64gc-unknown-linux-gnu/debug`

You can now test this binary in an emulated RISC-V environment, or hardware. It is important to remember that glibc versions can differ between Linux distribution versions. Ensure that your emulated environment has a glibc version equal to or greater than used by your build tools.

### Rust on RISC-V with the official toolchain

If packages are not available for RISC-V development on your OS, you may need to clone and build the tools yourself.

- Clone the git repo `https://github.com/riscv-collab/riscv-gnu-toolchain`
- Follow the instructions to install pre-requisites for your system.
- Run `./configure --prefix=/opt/riscv` - Change the prefix path as appropriate.
- Run `make linux`

The tools default to building for RV64GC, which is perfect for Rust on RISC-V Linux. When the build is done, you will have `/opt/riscv/bin/riscv64-unknown-linux-gnu-gcc` along with other tools and libraries.

Configure your Cargo project to use this as the linker.

### Testing with qemu and Debian

A great way to test RISC-V programs is by using qemu and Debian Quick Image Baker pre-baked images.

- Go to https://people.debian.org/~gio/dqib/ and download `Images for riscv64-virt`
- Uncompress the downloaded `artifacts.zip` file.
- Execute `./run.sh` to boot your RISC-V Linux system!

From here, you can either copy binaries into your vm over the local network or mounted filesystem.

### Testing with qemu and buildroot

[Buildroot](https://buildroot.org/) is also a great way to test RISC-V programs, especially in custom or constrained environments.

- Download buildroot or clone from `https://github.com/buildroot/buildroot`
- Run `make qemu_riscv64_virt_defconfig`
- Either run `make` to build the system or `make menuconfig` to tweak any settings.
- The resulting system will be built in `output/images/`
- Run `./start-qemu.sh` in the `output/images` directory.

Like with Debian on qemu, you can copy your test binary over the network, or use any other facility that qemu provides.

<div style="display: none; visibility: hidden;">
<hr><b>Next:</b> <a href="../end-to-end-encryption-with-rust#readme">End-to-End Encryption with Rust</a>
</div>
