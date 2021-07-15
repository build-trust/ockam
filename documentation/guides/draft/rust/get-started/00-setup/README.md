```yaml
title: Setup
```

# Setup

## Install Rust

We'll use Rust to build our example end-to-end encrypted application so, if
you don't have it already installed, please
[install it](https://www.rust-lang.org/tools/install).

The easiest way to do this is with `rustup`:

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Follow the instructions for a default installation.  When complete, ensure that
Cargo (Rust's package manager) has been added to the $PATH environment variable.
This can be verified with:

```
echo $PATH | grep cargo
````

If not found, manually add the $HOME/.cargo/bin directory to $PATH.

Note that if you install Rust with `rustup` and not with your operating system
package management tools, you should also verify that you have a C compiler and
linker/loader installed. Without these tools, `cargo run` will fail in subsequent
steps. This requirement can be verified if the following command provides output:

```
which cc
```

## Create a new Cargo package

With rust installed, let's use the rust package manager `cargo` to create a new
library package for the code we're about to write:

```
cargo new --lib ockam_get_started
```

This will generate a new library package in the `ockam_get_started` directory
that has the following generated files:

```
.
├── Cargo.toml
└── src
    └── lib.rs
```

We're "crating" a Cargo library package here because, during each step of this guide,
we'll write many simple binary programs. There are a few ways, in rust, to
create multiple binaries in a package. For this guide we'll create our binary
programs in the `examples` directory of our new project.

## Add Ockam

Add the `ockam` dependency to the newly generated `Cargo.toml` file:

```
[dependencies]
ockam = { version = "0" }
```
`ockam` is the main Ockam crate.

<div style="display: none; visibility: hidden;">
<hr><b>Next:</b> <a href="../01-node">01. Create an Ockam node</a>
</div>
