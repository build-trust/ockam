# Ockam implementation in Elixir

# Build

## Setup Erlang and Elixir

The Elixir website has [good instructions](https://elixir-lang.org/install.html)
on how to install elixir from various operating system package managers. This
usually also installs Erlang which is a prerequisite for running Elixir.

For development it is often helpful to have multiple versions of Erlang and
Elixir installed on your machine for testing, this is where [asdf](https://asdf-vm.com/)
can be helpful. There are asdf plugins for both [Erlang](https://github.com/asdf-vm/asdf-erlang)
and [Elixir](https://github.com/asdf-vm/asdf-elixir).

[Here's good guide](https://thinkingelixir.com/install-elixir-using-asdf/) on using
asdf to manage erlang and elixir version - [link](https://thinkingelixir.com/install-elixir-using-asdf/)

## CMake, Make and C toolchain

To compile the native code:
1. CMake should be install
2. GNU Make should available
3. C compiler toolchain should be available to CMake

## Get dependencies

```
mix deps.get
```

## Compile

```
mix compile
```

## Test

```
mix test
```
