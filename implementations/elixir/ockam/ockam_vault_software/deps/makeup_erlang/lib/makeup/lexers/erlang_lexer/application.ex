defmodule Makeup.Lexers.ErlangLexer.Application do
  @moduledoc false
  use Application

  alias Makeup.Registry
  alias Makeup.Lexers.ErlangLexer

  def start(_type, _args) do
    Registry.register_lexer(ErlangLexer,
      options: [],
      names: ["erlang", "erl"],
      extensions: ["erl", "hrl", "escript"]
    )

    Supervisor.start_link([], strategy: :one_for_one)
  end
end
