defmodule Makeup.Lexers.ElixirLexer.Application do
  @moduledoc false
  use Application

  alias Makeup.Registry
  alias Makeup.Lexers.ElixirLexer

  def start(_type, _args) do
    Registry.register_lexer(ElixirLexer,
      options: [],
      names: ["elixir", "iex"],
      extensions: ["ex", "exs"]
    )

    Supervisor.start_link([], strategy: :one_for_one)
  end
end
