# MakeupErlang

[![Build Status](https://travis-ci.org/tmbb/makeup_erlang.svg?branch=master)](https://travis-ci.org/tmbb/makeup_erlang)

A [Makeup](https://github.com/tmbb/makeup/) lexer for the `Erlang` language.

## Installation

Add `makeup_erlang` to your list of dependencies in `mix.exs`:

```elixir
def deps do
  [
    {:makeup_erlang, "~> 0.1.0"}
  ]
end
```

The lexer will automatically register itself with `Makeup` for the languages `erlang` and `erl` 
as well as the extensions `.erl`, `.hrl` and `.escript`.