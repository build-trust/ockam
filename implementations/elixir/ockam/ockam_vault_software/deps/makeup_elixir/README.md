# MakeupElixir

[![Build Status](https://github.com/elixir-makeup/makeup_elixir/workflows/CI/badge.svg)](https://github.com/elixir-makeup/makeup_elixir/actions)

A [Makeup](https://github.com/elixir-makeup/makeup/) lexer for the Elixir language.

## Installation

Add `makeup_elixir` to your list of dependencies in `mix.exs`:

```elixir
def deps do
  [
    {:makeup_elixir, "~> 0.14.0"}
  ]
end
```

The lexer will be automatically registered in Makeup for
the languages "elixir" and "iex" as well as the extensions
".ex" and ".exs".
