# Ockam

Thank you for your interest in contributing to the Ockam open source projects.

Ockam is a collection of protocols and toolkits for building connected
systems that you can trust. This folder contains the Typescript implementation of Ockam.

Please read our community's [_Code of Conduct Covenant_][conduct] and our [contributing guidelines][contributing].

To start contributing to our Typescript code, clone the Ockam repo from Github and change your current directory
to `implementations/typescript`:

```
git clone git@github.com:build-trust/ockam.git
cd implementations/typescript
```

# Using `pnpm`

## Setup

`pnpm` is a package management tool for `typescript`. You can install it by following the instructions at https://pnpm.io/installation.
Once you have installed `pnpm`, you can install `Typescript` with `pnpm add typescript -D`.

## Build

In order to build the code, you first need to install the project dependencies with:

```
pnpm install
```

Then the code can be built with:

```
pnpm build
```

The code can also be cleaned with:

```
pnpm clean
```

## Test

Once you make some in a package and write some tests with [`jest`](https://jestjs.io/docs/getting-started), you can run them with:

```
pnpm test
```

(you might have to first run `pnpm clean; pnpm build`).

## Formatting

This command formats all the Typescript files, via the [`prettier`](https://prettier.io) plugin.

```
pnpm format
```

## Lint

Lint check can be executed with
```
pnpm lint
```

At the moment this task only checks if the code is properly formatted.

## Get Help

Ask a question on [Github Discussions](https://github.com/build-trust/ockam/discussions)

# Using `make`

Many `pnpm` commands have an equivalent support using `make`, at the root directory.
Here are a few examples, you can find more in [implementations/typescript/Makefile](./Makefile):

| Command                  | Description                                     |
|--------------------------|-------------------------------------------------|
| `make typescript_clean`  | clean build files                               |
| `make typescript_build`  | build all packages (install dependencies first) |
| `make typescript_test`   | run the tests, using `pnpm test`                |
| `make typescript_format` | format the code, using `pnpm format`            |
| `make typescript_lint`   | lint the code, using `pnpm lint`                |

Note that these commands don't need to use the `typescript_` prefix if you first `cd` into `implementations/typescript`. Then you
can directly call: `make build`, `make test`, etc...

# Using `nix`

Our [Nix](https://nixos.org) tooling can be found in `tools/nix` from the root path directory. This has the benefit to install _all_ the tools
needed to build, test and check the project. You first need to install `nix` by following the instructions [here](https://nixos.org/download#download-nix).

Then you can run any `make` command in a `nix` environment. For example here is how you can run the tests:

```
nix develop ./tools/nix#typescript --command make typescript_test
```

There is also a `make` shortcut to run any command in a `nix` environment:

```
# run the jest tests
make nix_typescript_test
```

[conduct]: https://github.com/build-trust/.github/blob/main/CODE_OF_CONDUCT.md
[contributing]: https://github.com/build-trust/.github/blob/main/CONTRIBUTING.md
