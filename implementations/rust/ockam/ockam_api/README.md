# ockam_api

[![crate][crate-image]][crate-link]
[![docs][docs-image]][docs-link]
[![license][license-image]][license-link]
[![discuss][discuss-image]][discuss-link]

Ockam is a library for building devices that communicate securely, privately
and trustfully with cloud services and other devices.

This crate supports the creation of a fully-featured Ockam Node
(see [`NodeManager`](https://github.com/build-trust/ockam/blob/2fc6d7714a4e54f8734c172ad6480fedc6e3629c/implementations/rust/ockam/ockam_api/src/nodes/service.rs#L87) in [`src/nodes/service.rs`](https://github.com/build-trust/ockam/blob/2fc6d7714a4e54f8734c172ad6480fedc6e3629c/implementations/rust/ockam/ockam_api/src/nodes/service.rs)).

## Configuration

A `NodeManager` maintains its configuration as a list of directories and files stored under
the `OCKAM_HOME` directory (`~/.ockam`) by default:
```shell
root
├─ credentials
│  ├─ c1.json
│  ├─ c2.json
│  └─ ...
├─ defaults
│  ├── credential -> ...
│  ├── identity -> ...
│  ├── node -> ...
│  └── vault -> ...
├─ identities
│  ├─ data
│  │  ├─ authenticated-storage.lmdb
│  │  └─ authenticated-storage.lmdb-lock
│  ├─ identity1.json
│  ├─ identity2.json
│  └─ ...
├─ nodes
│  ├─ node1
│  │  ├─ default_identity -> ...
│  │  ├─ default_vault -> ...
│  │  ├─ policies-storage.lmdb
│  │  ├─ policies-storage.lmdb-lock
│  │  ├─ setup.json
│  │  ├─ stderr.log
│  │  ├─ stdout.log
│  │  └─ version.log
│  ├─ node2
│  └─ ...
├─ projects
│  └─ default.json
├─ trust_contexts
│  └─ default.json
└─ vaults
   ├─ vault1.json
   ├─ vault2.json
   ├─ ...
   └─ data
      ├─ vault1.lmdb
      ├─ vault1.lmdb-lock
      ├─ vault2.lmdb
      ├─ vault2.lmdb-lock
      └─ ...
```
## `credentials`

Each file stored under the `credentials` directory contains the credential for a given identity.
Those files are created with the `ockam credential store` command. They are then read during the creation of
a secure channel to send the credentials to the other party

## `defaults`

This directory contains symlinks to other files or directories in order to specify which node,
identity, credential or vault must be considered as a default when running a command expecting those
inputs

## `identities`

This directory contains one file per identity and a data directory. An identity file is created
with the `ockam identity create` command or created by default for some commands (in that case the
`defaults/identity` symlink points to that identity). The identity file contains:

- the identity identifier
- the enrollment status for that identity

The `data` directory contains a LMDB database with other information about identities:
 - the credential attributes that have been verified for this identity. Those attributes are
   generally used in ABAC rules that are specified on secure channels. For example when sending messages
   via a secure channel and using the Orchestrator the `project` attribute will be checked and the LMDB database accessed

 - the list of key changes for each identity. These key changes are created (or updated) when an identity
   is created either by using the command line or by using the identity service.
   The key changes are accessed in order to get the latest public key associated to a given identity
   when checking its signature during the creation of a secure channel.
   They are also accessed to retrieve the key id associated to that key and then use a Vault to create a signature
   for an identity

Note: for each `.lmdb` file there is a corresponding `lmdb-lock` file which is used to control
the exclusive access to the LMDB database even if several OS processes are trying to modify it.
For example when several nodes are started using the same `NodeManager`.

## `nodes`

This directory contains:

 - symlinks to default values for the node: identity and vault
 - a database for ABAC policies
 - a setup file containing some configuration information for the node (is it an authority node?, what is the TCP listener address?,...).
   That file is created when a node is created and read again if the node is restarted
 - log files: for system errors and system outputs. The stdout.log file is where almost all the node logs are written
 - a version number for the configuration

## `projects`

This directory contains a list of files, one per project that was created, either the default project
or via the `ockam project create` command. A project file contains:

 - the project identifier and the space it belongs to
 - the authority used by that project (identity, route)
 - the configuration for the project plugins

## `trust_context`

This directory contains a list of files, one per trust context. A trust context can created with
the `ockam trust_context create` command. It can then be referred to during the creation of a
secure channel as a way to specify which authority can attest to the validity of which attributes

## `vaults`

This directory contains one file per vault that is either created by default or with the `ockam vault create`
command. That file contains the configuration for the vault, which for now consists only in
declaring if the vault is backed by an AWS KMS or not.

The rest of the vault data is stored in an LMDB database under the `data` directory with one `.lmdb`
file per vault. A vault contains secrets which are generally used during the creation of secure
channels to sign or encrypt data involved in the handshake.


## Usage

Add this to your `Cargo.toml`:

```
[dependencies]
ockam_api = "0.41.0"
```

## License

This code is licensed under the terms of the [Apache License 2.0][license-link].

[main-ockam-crate-link]: https://crates.io/crates/ockam

[crate-image]: https://img.shields.io/crates/v/ockam_api.svg
[crate-link]: https://crates.io/crates/ockam_api

[docs-image]: https://docs.rs/ockam_api/badge.svg
[docs-link]: https://docs.rs/ockam_api

[license-image]: https://img.shields.io/badge/License-Apache%202.0-green.svg
[license-link]: https://github.com/build-trust/ockam/blob/HEAD/LICENSE

[discuss-image]: https://img.shields.io/badge/Discuss-Github%20Discussions-ff70b4.svg
[discuss-link]: https://github.com/build-trust/ockam/discussions
