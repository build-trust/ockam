# 3. Crate Feature Flags

Date: 2021-09-20

## Status

Proposed

## Context

Ockam wishes to be usable (and useful) in resource-constrained environments, including on embedded microcontrollers without a typical OS, and/or where dynamic memory allocation is not possible (either because resource limitations which make it impractical, or because of industrial standards which forbid it). As a result, our Rust crates must function in the following configurations:

1. `no_std`: Requires that memory be allocated statically at compile time, and no global allocator is available. Some features might be enabled in this mode for stack based collection usage like `heapless`. This is stack only mode.
1. `no_std+alloc`: Standard collections provided by `alloc` can be used but they do not use the standard allocator. This allows the use of heap allocation via custom allocator(s).
1. `std`: normal Rust. This is the default mode.

### Background: The Rust Standard Libraries

The public API of Rust's standard library is split into 3 parts.

1. `libcore` (e.g. `core::*`): This is the base of the Rust standard library, and cannot generally be disabled. It is limited to functionality which:
    - Does not perform any dynamic memory allocation, or require the existence of a global memory allocator.
    - Has no dependencies on external libraries or functionality, such as that provided by the system C standard library.
    - Makes no assumptions about the environment where it runs, beyond what is guaranteed by the target architecture and instruction set.
        - For example, it can perform atomic operations, but only if the target instruction set can do so natively.
        - Concretely, it can't contain `#[cfg(target_os = "...")]` (`target_family`, `unix`, `windows`, and a few others are also among the `cfg`s considered off-limits) but is allowed to use ones such as `#[cfg(target_arch = "...")]`.
    - More broadly, it does not require the existence of an OS, and has no access to things like files or threads.

2. `liballoc` (e.g. `alloc::*`): This is a superset of `libcore` which is allowed to perform dynamic memory allocation from a global memory allocator.
    - The allocator may be user-provided (via the `#[global_allocator]` attribute), rather than the one provided by an OS or `libc`.
    - This mostly contains collection types, such as `Vec`, `String`, `BTreeMap`, etc.
    - Beyond the ability to perform global memory allocation, it has the same restrictions as `libcore`:
        - No dependencies on system libraries.
        - No assumptions about target environment beyond what is guaranteed by the target architecture.

3. `libstd`, (e.g. `std::*`): This depends on (and is a superset of) both `liballoc` and `libcore`. It is what is used by default.
    - Broadly speaking, it requires an OS that supports files and threads.
        - There are exceptions here, in that a few targets exist which have `libstd` support but just return errors when this functionality is used.
    - It can assume whatever it needs to about the target environment.
        - It's allowed to contain `#[cfg(target_os = "...")]` statements internally.
        - It's allowed to link against system-specific libraries on the OS (`libc`, `libm`, `libSystem.dylib`, `kernel32.dll`, ...).
        - It can even place requirements on the OS version in use (to forbid Windows XP, for example)
    - As it depends on `liballoc`, `libstd` is allowed to perform memory allocation anywhere it wants.
        - If a `#[global_allocator]` is configured, it will do so out of that allocator
        - If no `#[global_allocator]` is configured, then `libstd` will provide one based on the OS's default allocator.
            - In the rare cases where `libstd` supports a target without a system-provided allocator, `libstd` will provide a default written in Rust (currently, it uses a Rust [port of `dlmalloc`](https://crates.io/crates/dlmalloc) for these situations).

As you may note, these are analogous to the three configurations we wish to support, but there is an important difference:

We should do not adopt the restriction that `libcore` and `liballoc` has around target-specific checks and `cfg`s and such. We favor practicality over purity, and without this, we would be completely unable to provide useful functionality for embedded environments.

## Decision

To reflect these three scenarios, we will use three cargo features:
1. `feature = "std"`, which allows full use of `libstd`. Mirroring the Rust stdlib, it implies `feature = "alloc"`.
2. `feature = "alloc"`, which allows use of an allocator. As  `feature = "std"`.
3. `feature = "no_std"` is used to add dependencies which are needed only in `no_std` configurations.
    - Generally, the user is going to have to provide either `feature = "std"` or `feature = "no_std"`.

Certain crates are not even possible in alloc and stack only modes like transport. These crates should still follow these guidelines to the best of their extent possible.

There are three files to apply the guidelines to:

    Cargo.toml
    lib.rs
    README.md

### Cargo.toml

In the Cargo.toml file, the following 4 lines should be directly under the `[features]` section

```toml
default = ["std"]
std = ["ockam_core/std", "alloc"]
alloc = ["ockam_core/alloc"]
no_std = ["ockam_core/no_std"]
```

The last two lines do not apply to crates that only function in standard mode.

### lib.rs

In the lib.rs file, add the following lines:

```rust
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
#[macro_use]
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

```rust
// only compile for std
#[cfg(feature = "std")]

// compile for any environment with an allocator,
// either `no_std` or `no_std+alloc`.
#[cfg(feature = "alloc")]

// compile for any `no_std` build,
// either `no_std` or `no_std+alloc`.
#[cfg(not(feature = "std"))]

// compile for `no_std` (without `alloc`) only.
#[cfg(not(feature = "alloc"))]

// Compile for no_std+alloc only.
#[cfg(all(feature = "alloc", not(feature = "std")))]
```

## Decision

To avoid compromising the readability of code through the over-use of conditional compilation attributes we decided to create the `ockam_core::compat` module.

It's function is primarily to act as a wrapper around items in the `std::` library that are not available in the `core::` namespace.

The rules of use are as follows:

1. always prefer `core::<mod>` over `std::<mod>` where available. (e.g. `std::fmt::Result` -> `core::fmt::Result`)
2. otherwise, use `ockam_core::compat::<mod>` equivalents where available. (e.g. `std::sync::Arc -> ockam_core::compat::sync::Arc`)
3. if you need to add new items to compat, follow the originating namespace. (e.g. `compat::vec::Vec` and not `compat::Vec`)
4. if none of the above apply use the conditional compilation attributes as documented above

In the future, we may move `ockam_core::compat` into a dedicated crate in order to reduce feature fragility, however prior to general availability, this is probably fine).
