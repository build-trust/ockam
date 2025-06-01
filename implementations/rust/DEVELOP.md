# Ockam

Thank you for your interest in contributing to the Ockam open source projects.

Please read our community's [*Code of Conduct Covenant*][conduct] and our [contributing guidelines][contributing].

To start contributing to our rust code, clone the Ockam repo from Github and change your current directory
to `implementations/rust`:

```
git clone git@github.com:build-trust/ockam.git
cd implementations/rust
```

# Get Help

Ask a question on [Discord](https://discord.ockam.io).

# Using `cargo`

## Setup

If you don't already have it, you will need Rust stable and nightly toolchains installed.
To get them install [rustup](https://rustup.rs) and then use it setup the `stable` and `nightly` rust toolchains:

```
rustup toolchain install stable
rustup toolchain install nightly
```

Refer Rust [documentation][rustup-manage-versions] on managing and updating rust versions.

## Build

The code can be built with:

```
cargo build
```

This will produce an `ockam` executable at `target/debug/ockam`.

You can also typecheck the code, as you modify it, with:

```
cargo watch -x check
```

or, if you want to include the test code:

```
cargo watch -x "check --tests"
```

### `no_std`

The `ockam` crate has a Cargo feature named `"std"` that is enabled by default.
However it can also be built in a `no_std` context with:

```
cargo build --target thumbv7em-none-eabihf --package ockam --no-default-features --features 'no_std alloc software_vault'
```

## Test

Once you make some changes in a crate and write some tests, you can run them with:

```
cargo test
```

### Concurrent tests

There is also a way to run all the tests concurrently, to run them faster, with `nextest`:

```
cargo --config-file tools/nextest/.config/nextest.toml run --no-fail-fast
```

`--no-fail-fast` runs all the tests, regardless of failures.

### BATS tests

The Ockam system tests are implemented using a command-line test tool: [bats](https://bats-core.readthedocs.io).
Please
consult [the `bats` installation instructions to install it on your system](https://bats-core.readthedocs.io/en/stable/installation.html).

Once `bats` is installed you can run the `bats` test suite with:

```
# --jobs 4 run the tests concurrently using 4 threads
bats implementations/rust/ockam/ockam_command/tests/bats --jobs 4
```

The `implementations/rust/ockam/ockam_command/tests/bats` directory contains several tests files, covering different
functionalities. You can specify which file you want to test and even add a filter for the subset of tests you want to
run:

```
# run only the CRUD tests in the vault test suite
bats implementations/rust/ockam/ockam_command/tests/bats/vault.bats --jobs 4 --filter "CRUD"
```

## Lint

To validate that the new code you've added is formatted according to our project conventions:

```
cargo fmt --check
```

You can ask cargo to automatically fix any formatting inconsistencies by running:

```
cargo fmt
```

`clippy` is a Cargo plugin that can catch many common mistakes. You can run on the Ockam code with:

```
cargo clippy --no-deps --all-targets -- -D warnings
```

## Documentation

Generate rust documentation:

```
cargo doc
```

### README files

The `README.md` files for a given crate is generated from the documentation header in the top-level `lib.rs` file,
using the `cargo readme` plugin.

That plugin must be installed with `cargo install cargo-readme`. Then a `README` file in a given crate can be updated
with:

```
# update the README file for the ockam_identity crate
cd implementations/rust/ockam_identity
cargo readme --project-root ockam_identity --template ../README.tpl -o README.md
```

## Code Coverage

Get a code coverage report:

```
# install the grcov binary
cargo install grcov

# run the cargo tests with a coverage profile
# this needs to run with +nightly
export CARGO_INCREMENTAL=0                                                                                                                                                                                                                                                                                                                                                                            ockam dotfiles flox/default default
export RUSTFLAGS="-Zprofile -Ccodegen-units=1 -Copt-level=0 -Clink-dead-code -Coverflow-checks=off -Zpanic_abort_tests"
cargo +nighlty test --profile coverage

# generate the report
~/.cargo/bin/grcov --llvm . -s . --binary-path ./target/debug/ -t html --branch --ignore-not-existing -o ./target/debug/coverage/

# open the report
open target/debug/coverage/index.html
```

It seems that some directories are not included in coverage data when running `cargo test`.
In order to still get a coverage report for those directories you can use `cargo nextest` instead:

```
# install the llvm-cov cargo plugin
cargo install cargo-llvm-cov

# run the tests and produce an HTML report
cargo llvm-cov nextest -p ockam_api --html

# open the coverage report
open target/llvm-cov/html/index.html
```

## Crate Dependency Graph

Generate a crate dependency graph of Ockam's crates:

```
cargo install cargo-depgraph
cargo depgraph --workspace-only | dot -Tpng > graph.png
```

## Module Dependency Graph

Generate a module dependency graph:

```
cargo install cargo-modules
cargo modules --orphans graph | dot -Tpng > modules.png
```

## Dependency Licenses

See licenses used by all dependencies:

```
cargo install cargo-license
cargo license
```

See a unique list of all dependencies, this is useful in confirming that we are only adding dependencies that have
permissive license like an Apache, MIT or BSD variant.

```
cargo license --json | jq ".[] | .license" | sort | uniq
```

# Using `make`

Many `cargo` commands have an equivalent support using `make`. Here are a few examples, you can find more
in [implementations/rust/Makefile](./Makefile):

| Command                       | Description                                                                                                                       |
|-------------------------------|-----------------------------------------------------------------------------------------------------------------------------------|
| `make rust_clean`             | clean build files                                                                                                                 |
| `make rust_build`             | build all crates                                                                                                                  |
| `make rust_test`              | run the tests, using `cargo test`                                                                                                 |
| `make rust_nexttest`          | run the tests, using `cargo nextest`                                                                                              |
| `make rust_bats`              | run the `bats` test suite                                                                                                         |
| `make rust_bats_local_vault`  | run the local `bats` `vault` test suite (replace `local` with `orchestrator` or `serial` and `vault` with another bats file name. |
| `make rust_update_readmes`    | update the `README` files in all crates, based on the documentation header in `<crate>/src/lib.rs`                                |
| `make rust_cargo_fmt`         | format the code                                                                                                                   |
| `make rust_lint`              | run all the code lints                                                                                                            |
| `make rust_lint_clippy`       | run the clippy lints                                                                                                              |
| `make rust_lint_cargo_readme` | check that the README files are up to date                                                                                        |
| `make rust_check_no_std`      | check that the `ockam` crate can be compiled with the `no_std` feature                                                            |

Note that these commands don't need to use the `rust_` prefix if you first `cd` into `implementations/rust`. Then you
can directly call: `make build`, `make test`, etc...

# Using `nix`

Our [Nix](https://nixos.org) tooling can be found in `tools/nix` from the root path directory. This has the benefit to
install *all* the tools
needed to build, test and check the project. You first need to install `nix` by following the
instructions [here](https://nixos.org/download#download-nix).

Then you can run any `make` command in a `nix` environment. For example here is how you can run the tests:

```
nix develop ./tools/nix#rust --command make rust_test
```

There is also a `make` shortcut to run any command in a `nix` environment:

```
# run the cargo tests
make nix_rust_test

# run the bats tests
make nix_rust_bats
```

# Using an IDE

We recommend using [RustRover](https://www.jetbrains.com/rust) or VSCode
with [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
to benefit from code navigation, refactoring and automated formatting.

# Rust Performance Guidelines

## Intro

This is a quick guideline to avoid the most common performance pitfalls in rust.
Although performance is important, it's not the only thing to consider when writing code. This guideline should be closely applied only in hot-paths, where performance is crucial, and more loosely applied in cold-paths, where readability and maintainability are more important.

## Memory Allocations
Frequent memory allocations can degrade performance either for the sheer number of allocations or for the consequent memory fragmentation. Most of the guidelines here are to avoid unnecessary allocations and copies. For Ockam Command, the typical memory fragmentation symptoms are unstable or degraded performance.

## Tools
There are some tools that can help you to profile Ockam Command portals. See [here](tools/profile/README.md) for more information.

## Guidelines

### Owned Parameters
If the function needs an owned copy of the element, ask an owned instance directly in the parameter. This will allow the caller to decide if it wants to clone the element or give its own.

```rust
// Bad
fn example(item: &Type) {
  self.item = item.clone();
}

// Good
fn example(item: Type) {
  self.item = item;
}
```

### Returning a reference
Avoid returning a cloned value when a reference is enough. This will avoid unnecessary allocations. It'll be the caller's responsibility to clone the value when needed.

```rust
// Bad
fn get_value(&self) -> Type {
  self.value.clone()
}

// Good
fn get_value(&self) -> &Type {
  &self.value
}
```

### Borrowed Parameters
If the function doesn't need to own the instance, ask for a borrowed instance in the parameter.

```rust
// Bad
fn example(item: Type) {
  self.map.get(&item);
}

// Good
fn example(item: &Type) {
  self.map.get(item);
}
```

### Allow direct field access
By allowing direct access to a structure fields, it can be "deconstructed" and used directly, avoiding unnecessary allocations.

```rust
// Bad
struct MyStruct {
    field: Type,
}

impl MyStruct {
    fn get_field(&self) -> &Type {
        &self.field
    }
}

fn example(my_struct: MyStruct) {
    let field = my_struct.get_field().clone();
    do_something(field);
}

// Good
struct MyStruct {
    pub field: Type,
}

fn example(my_struct: MyStruct) {
    do_something(my_struct.field);
}
```

### `ok_or` Errors allocation
Memory allocations for errors should only happen when an error is actually returned. To avoid unnecessary allocations, use `ok_or_else` instead of `ok_or`.

```rust
// Bad
let value = some_function().ok_or(Error::new("Error"));

// Good
let value = some_function().ok_or_else(|| Error::new("Error"));
```

### Reuse in loops
When possible, reuse instances in loops to avoid unnecessary allocations.

```rust
// Bad
loop {
  let value = Vec::new();
  // do something with value
}

// Good
let mut value = Vec::new();
loop {
  value.clear();
  // do something with value
}
```

### Pre-compute the size
Not providing the size to a vector will cause re-allocations. A re-allocation is a request to enlarge the existing allocation, when it's not possible it'll cause a copy of the existing data to a new allocation. This is exponentially expensive as the size grows. To avoid this, pre-compute the size of the vector when possible, Cbor structures have the possibility to pre-compute the size before serialization.

```rust
// Bad
let mut vec = Vec::new();
for i in 0..100 {
  vec.push(i);
}

// Good
let mut vec = Vec::with_capacity(100);
for i in 0..100 {
  vec.push(i);
}
```

However, an empty `Vec` has zero capacity and doesn't trigger an allocation until the first element is pushed. If you first need to create a `Vec` and then populate it, use `reserve` to pre-allocate the necessary space.

```rust
struct MyStruct {
    vec: Vec<i32>,
}

// Bad
impl MyStruct {
    fn new() -> Self {
        MyStruct {
            // no need to pre-allocate here
            vec: Vec::with_capacity(100),
        }
    }

    fn populate(&mut self) {
        for i in 0..100 {
            self.vec.push(i);
        }
    }
}

// Good
impl MyStruct {
    fn new() -> Self {
        MyStruct {
            // no extra allocation here
            vec: Vec::new(),
        }
    }

    fn populate(&mut self) {
        self.vec.reserve(100);
        for i in 0..100 {
            self.vec.push(i);
        }
    }
}
```

### Avoid `async` Traits
The `async_trait` implementation allocates a future every time a method is called.
When an `async_trait` is being called very frequently (100+/s), consider the usage of synchronous traits, or a template instead.

### Cow
Sometimes when a function needs to return a value that can be either owned or borrowed, `Cow` can be used to avoid cloning the borrowed value. This also requires the usage of a lifetime and may increase the complexity of the code, use it only when necessary.

```rust
// Bad
struct MyStruct {
    value: Type,
}

fn create_struct_owned(value: Type) -> MyStruct {
    MyStruct {
        value,
    }
}
fn create_struct_borrowed(value: &Type) -> MyStruct {
    MyStruct {
        value: value.clone(),
    }
}

// Good
struct MyStruct<'a> {
    value: Cow<'a, Type>,
}

fn create_struct_owned(value: Type) -> MyStruct {
    MyStruct {
        value: Cow::Owned(value),
    }
}

fn create_struct_borrowed(value: &Type) -> MyStruct {
    MyStruct {
        value: Cow::Borrowed(value),
    }
}
```

### Avoid overly complex data structures
`HashMap`, `HashSet`, and other complex data structures can be expensive in terms of memory and relative slow. When the number of elements is small (10s), consider using a simple vector instead.

```rust
// Bad
struct MyStruct {
    plugins: HashMap<String, Type>,
}

impl MyStruct {
    fn new() -> Self {
        let mut plugins = HashMap::new();
        plugins.insert("plugin1".to_string(), Type::new());
        plugins.insert("plugin2".to_string(), Type::new());
        plugins.insert("plugin3".to_string(), Type::new());

        MyStruct {
            plugins
        }
    }
}

// Good
struct MyStruct {
    plugins: Vec<(String, Type)>,
}

impl MyStruct {
    fn new() -> Self {
        let plugins = vec![
            ("plugin1".to_string(), Type::new()),
            ("plugin2".to_string(), Type::new()),
            ("plugin3".to_string(), Type::new()),
        ];

        MyStruct {
            plugins
        }
    }
}
```

# Telemetry

Ockam commands and nodes generate telemetry data in the form of Opentelemetry logs and spans.
This telemetry data is collected via [an Opentelemetry collector](https://opentelemetry.io/docs/collector). That
collector serves as a central point of collection and can forward this data to a variety of backends, such as:

- S3 for long-term storage.
- Observability systems like Honeycomb, DataClickHouse, etc... for analysis and visualization.

# Configuration

When the `OCKAM_OPENTELEMETRY_EXPORT` environment variable is set to `true`, there are various ways to send telemetry
data to the collector:

1. Via a secure channel to your project and then to the collector (the default).
1. Via a secure channel to your project's authority node and then to the collector.
1. Via a secure channel to an arbitrary Ockam node and then to the collector.
1. Directly via HTTP.

## Sending telemetry data directly via a secure channel to the project

This behaviour is controlled by the `OCKAM_TELEMETRY_EXPORT_VIA_PROJECT` environment variable (the default is `true`).

This mode is only active if a default project can be detected locally and is accessible via a secure channel.
In that case, the telemetry data is sent as Ockam messages to the project node and then forwarded to the collector.

### Services configuration

The project node must be started with the following configuration:

```elixir
  config :ockam_services,
    services:
      {:grpc_forwarder,
      [
        address: "grpc_forwarder",
        grpc_endpoint: "http://opentelemetry-collector:4317",
        authorization: [{Ockam.Worker.Authorization, :from_secure_channel}]
      ]}
```

The value of `grpc_endpoint` must be set to the endpoint of an accessible OpenTelemetry collector.

## Sending telemetry data directly via a secure channel to the authority node

This behaviour is controlled by the `OCKAM_TELEMETRY_EXPORT_VIA_AUTHORITY` environment variable (the default is
`false`).

This mode is only active if a default project can be detected locally and is accessible via a secure channel.
Then, the telemetry data is sent as Ockam messages to the project's authority node and forwarded to the collector.

### Configuration

In order for the forwarding to work, the authority node be configured with the `OCKAM_OPENTELEMETRY_ENDPOINT` set to
the collector endpoint URL, for example `http://opentelemetry-collector:4317`.

## Sending telemetry data directly via a secure channel to an arbitrary Ockam node

This behaviour is controlled by the setting of two environment variables:

- `OCKAM_TELEMETRY_EXPORT_NODE_ROUTE` a route to the node to connect. For example:
  `/dnsaddr/localhost/tcp/4000/secure/api`. Note that the presence of `secure/api` in the address is what's triggering
  the creation of a secure channel and allows a node to send telemetry data without having to expose an OpenTelemetry
  collector residing in your private network.
- `OCKAM_TELEMETRY_EXPORT_NODE_IDENTIFIER` the identifier of the node that we are connecting to.
- `OCKAM_TELEMETRY_EXPORT_NODE_FORWARDER_SERVICE` the address of the `GrpcForwarder` service started on the remote node.
  The default is `grpc_forwarder` (see the project configuration above where that name is used to start the
  `GrpcForwarder` for example)

## Sending telemetry data directly via HTTP

In order to do this you need to set the following environment variables:

- `OCKAM_TELEMETRY_EXPORT=true`: this is the default value.
- `OCKAM_OPENTELEMETRY_ENDPOINT=http://opentelemetry-collector:4317`, assuming that your OpenTelemetry collector is
  running a gRPC endpoint on port 4317.

The `receivers` section of your collector configuration file should look like this:

```yaml
receivers:
  otlp:
    protocols:
      grpc:
        endpoint: "0.0.0.0:4317"
```

Notes:

- The `0.0.0.0` address, which enables the accessibility of the port 4317 on all network interfaces.
  This is in particular required if you deploy an Opentelemetry collector in a container.
- It is also possible to use a HTTPs address for the endpoint.

## Cutoff times

When a command is executed, log messages and spans cumulated in batches and those batches are then sent to the
collector. A few environment variables can be used to control the sending of these batches:

- `OCKAM_SPAN_EXPORT_TIMEOUT`: Timeout for trying to export spans. Default value: `5s`.
- `OCKAM_SPAN_EXPORT_QUEUE_SIZE`: Size of the queue used to store spans before they are sent. When the queue is full,
  spans are dropped. Default value: `32768`
- `OCKAM_FOREGROUND_SPAN_EXPORT_SCHEDULED_DELAY`: Maximum duration between the sending of two batches of spans. Default
  value: `1000s` (this value is high to avoid a deadlock in the tracing library).
- `OCKAM_FOREGROUND_SPAN_EXPORT_CUTOFF`: Cutoff time for sending a span batch, without waiting for a response. Default
  value: `3s`.

The same environment variables are available for logs instead of spans (replace `SPAN` by `LOG`) and for
a background node instead of a foreground node or command (replace `FOREGROUND` by `BACKGROUND`).

Default values are the same except for `OCKAM_BACKGROUND_SPAN/LOG_EXPORT_SCHEDULED_DELAY` which is set to `5s`.

# Debugging

The variable `OCKAM_OPENTELEMETRY_EXPORT_DEBUG` can be set to `true` to display info/debug and error messages related to
the setup of the telemetry sub-system.


[conduct]: https://github.com/build-trust/.github/blob/main/CODE_OF_CONDUCT.md

[contributing]: https://github.com/build-trust/.github/blob/main/CONTRIBUTING.md

[rustup-manage-versions]: https://doc.rust-lang.org/nightly/edition-guide/rust-2018/rustup-for-managing-rust-versions.html#rustup-for-managing-rust-versions
