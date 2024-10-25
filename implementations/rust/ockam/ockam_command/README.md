# ockam_command

[![crate][crate-image]][crate-link]
[![docs][docs-image]][docs-link]
[![license][license-image]][license-link]
[![discuss][discuss-image]][discuss-link]

Ockam is a library for building devices that communicate securely, privately
and trustfully with cloud services and other devices.

This crate provides the ockam command line application to:
 - start Ockam nodes and interact with them
 - manage projects and spaces hosted within the Ockam Orchestrator

For more information please visit the [command guide](https://docs.ockam.io/reference/command)

### Instructions on how to install Ockam Command
1. You can install Ockam Command pre-built binary using these [steps](https://docs.ockam.io/#quick-start). You can run the following command in your terminal to install the pre-built binary:

    ```bash
    curl --proto '=https' --tlsv1.2 -sSfL https://install.command.ockam.io | bash
    ```

1. To build Ockam Command from source, fork the [repo](https://github.com/build-trust/ockam), and then clone it to your machine. Open a terminal and go to the folder that you just cloned the repo into. Then run the following to install `ockam` so that you can run it from the command line.

    ```bash
    cd implementations/rust/ockam/ockam_command && cargo install --path .
    ```

## Usage

Add this to your `Cargo.toml`:

```
[dependencies]
ockam_command = "0.140.0"
```

## License

This code is licensed under the terms of the [Apache License 2.0][license-link].

[main-ockam-crate-link]: https://crates.io/crates/ockam

[crate-image]: https://img.shields.io/crates/v/ockam_command.svg
[crate-link]: https://crates.io/crates/ockam_command

[docs-image]: https://docs.rs/ockam_command/badge.svg
[docs-link]: https://docs.rs/ockam_command

[license-image]: https://img.shields.io/badge/License-Apache%202.0-green.svg
[license-link]: https://github.com/build-trust/ockam/blob/HEAD/LICENSE

[discuss-image]: https://img.shields.io/badge/Discuss-Github%20Discussions-ff70b4.svg
[discuss-link]: https://github.com/build-trust/ockam/discussions
