[![Hex version badge](https://img.shields.io/hexpm/v/erlex.svg)](https://hex.pm/packages/erlex)
[![License badge](https://img.shields.io/hexpm/l/erlex.svg)](https://github.com/asummers/erlex/blob/master/LICENSE.md)
[![Build status badge](https://img.shields.io/circleci/project/github/asummers/erlex/master.svg)](https://circleci.com/gh/asummers/erlex/tree/master)
[![Code coverage badge](https://img.shields.io/codecov/c/github/asummers/erlex/master.svg)](https://codecov.io/gh/asummers/erlex/branch/master)

# Erlex

Convert Erlang style structs and error messages to equivalent Elixir.

Useful for pretty printing things like Dialyzer errors and Observer
state. NOTE: Because this code calls the Elixir formatter, it requires
Elixir 1.6+.

## Documentation
[Hex Docs](https://hexdocs.pm/erlex).

## Changelog

Check out the [Changelog](https://github.com/asummers/erlex/blob/master/CHANGELOG.md).

## Installation

The package can be installed from Hex by adding `erlex` to your list
of dependencies in `mix.exs`:

```elixir
def deps do
  [
    {:erlex, "~> 0.2"},
  ]
end
```

## Usage

Invoke `Erlex.pretty_print/1` with the input string.

```elixir
iex> str = ~S"('Elixir.Plug.Conn':t(),binary() | atom(),'Elixir.Keyword':t() | map()) -> 'Elixir.Plug.Conn':t()"
iex> Erlex.pretty_print(str)
(Plug.Conn.t(), binary() | atom(), Keyword.t() | map()) :: Plug.Conn.t()
```

While the lion's share of the work is done via invoking
`Erlex.pretty_print/1`, other higher order functions exist for further
formatting certain messages by running through the Elixir formatter.
Because we know the previous example is a type, we can invoke the
`Erlex.pretty_print_contract/1` function, which would format that
appropriately for very long lines.

```elixir
iex> str = ~S"('Elixir.Plug.Conn':t(),binary() | atom(),'Elixir.Keyword':t() | map(), map() | atom(), non_neg_integer(), binary(), binary(), binary(), binary(), binary()) -> 'Elixir.Plug.Conn':t()"
iex> Erlex.pretty_print_contract(str)
(
  Plug.Conn.t(),
  binary() | atom(),
  Keyword.t() | map(),
  map() | atom(),
  non_neg_integer(),
  binary(),
  binary(),
  binary(),
  binary(),
  binary()
) :: Plug.Conn.t()
```
## Contributing

We welcome contributions of all kinds! To get started, click [here](https://github.com/asummers/erlex/blob/master/CONTRIBUTING.md).

## Code of Conduct

Be sure to read and follow the [code of conduct](https://github.com/asummers/erlex/blob/master/code-of-conduct.md).
