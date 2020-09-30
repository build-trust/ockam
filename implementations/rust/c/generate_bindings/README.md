## Purpose
This target is created to generate binding.rs file for c library components.
It's meant to be run once after any c code change, resulting binding.rs should be committed. This target should be
disabled by default because it's slow and brings additional dependencies.

## Steps to generate bindings
1. ```cd implementations/rust/c/bindings```
1. ```cargo build```
1. Add following code at the top of rust/c/bindings/src/bindings.rs
    ```rust
    #![allow(non_camel_case_types)]
    #![allow(improper_ctypes)]
    ```
1. Commit bindings.rs