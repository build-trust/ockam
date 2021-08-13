# 3. Crate Feature Flags

Date: 2021-02-10

## Status

Proposed

## Context

Ockam crates need guidelines for handling the following conditions in Rust

1. `no_std` Requires that memory be allocated statically at compile time. Some features might be enabled in this mode for stack based collection usage like `heapless`. This is stack only mode.
1. `no_std+alloc` Standard collections provided by `alloc` can be used but they do not use the standard allocator. This allows the use of heap allocation via custom allocator(s).
1. `std` normal Rust. This is standard mode.

This document describes the various scenarios that can happen and how to address them. It also describes what is needed for end users of the system to understand what they get and ensure consistency across ockam crates.


## Decision

Certain crates are not even possible in alloc and stack only modes like transport. These crates should still follow these guidelines to the best of their extent possible.

There are three files to apply the guidelines to:

    Cargo.toml
    lib.rs
    README.md

### Cargo.toml

In the Cargo.toml file, the following 4 lines should be directly under the `[features]` section

```toml
default = ["std"]
std = ["ockam_core/std"]
no_std = ["ockam_core/no_std"]
alloc = ["ockam_core/alloc"]
```

The last two lines do not apply to crates that only function in standard mode.

### lib.rs

In the lib.rs file, add the following lines:

```rust
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate core;

#[cfg(feature = "alloc")]
extern crate alloc;
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
    ockam = { version = "0.1.0", default-features = false, features = ["no_std", "alloc"] }
    ```

no\_std without alloc:
    ```
    [dependencies]
    ockam = { version = "0.1.0", default-features = false, features = ["no_std"] }
    ```

## Decision

To effect conditional compilation on `std`, `no_std+alloc` and `no_std` builds the following attributes can be used:

    // only compile for std
    #[cfg(feature = "std")]

    // only compile for no_std
    #[cfg(not(feature = "std"))]

    // only compile for no_std+alloc
    #[cfg(feature = "alloc")]

    // compile for no_std and no_std_+alloc
    #[cfg(any(not(feature = "std"), feature = "alloc"))]

## Decision

To avoid compromising the readability of code through the over-use of conditional compilation attributes we decided to create the `ockam_core::compat` module.

It's function is primarily to act as a wrapper around items in the `std::` library that are not available in the `core::` namespace.

The rules of use are as follows:

1. always prefer `core::<mod>` over `std::<mod>` where available. (e.g. `std::fmt::Result` -> `core::fmt::Result`)
2. otherwise, use `ockam_core::compat::<mod>` equivalents where available. (e.g. `std::sync::Arc -> ockam_core::compat::sync::Arc`)
3. if you need to add new items to compat, follow the originating namespace. (e.g. `compat::vec::Vec` and not `compat::Vec`)
4. if none of the above apply use the conditional compilation attributes as documented above
