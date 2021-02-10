# 3. Crate Feature Flags

Date: 2021-02-10

## Status

Proposed

## Context

Ockam crates need guidelines for handling the following conditions in Rust

1. `no-std` and no `alloc`. Instead everything must be specified at compile time. Some features might be enabled in this mode for stack based collection usage like `heapless`. This is stack only mode.
1. `no-std` with `alloc`. Collections available with `alloc` can be used but doesn't use the standard allocator. This is alloc only mode.
1. `std` normal Rust. This is standard mode.

This document describes the various scenarios that can happen and how to address them. It also describes what is needed for end users of the system to understand what they get and ensure consistency across ockam crates.


## Decision

Certain crates are not even possible in alloc and stack only modes like transport. These crates should still follow these guidelines to the best of their extend possible. There are three files to apply the guidelines: Cargo.toml, lib.rs, and README.md.

### Cargo.toml

In the Cargo.toml file, the following 4 lines should be directly under the `[features]` section

```toml
default = ["std"]
std = ["alloc", "ockam_core/std"]
alloc = []
no_std = ["ockam_core/no_std"]
```

The last two lines do not apply to crates that only function in standard mode.

### lib.rs

In the lib.rs file, add the following line

```rust
#![no_std]
```

### README.md

In the usage section of the README.md add the following

std:
    ```
    [dependencies]
    ockam = "0.1.0"
    ```

no\_std with alloc:
    ```
    [dependencies]
    ockam = { version = "0.1.0", default-features = false, features = ["no-std", "alloc"] }
    ```

no\_std without alloc:
    ```
    [dependencies]
    ockam = { version = "0.1.0", default-features = false, features = ["no-std"] }
    ```

