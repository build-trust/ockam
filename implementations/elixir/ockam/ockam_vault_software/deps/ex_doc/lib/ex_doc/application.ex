defmodule ExDoc.Application do
  @moduledoc false
  use Application

  def start(_type, _args) do
    Makeup.Registry.register_lexer(ExDoc.ShellLexer,
      options: [],
      names: ["shell", "console", "sh", "bash", "zsh"],
      extensions: []
    )

    # Load applications so we can find their modules in docs
    Enum.each([:eex, :ex_unit, :iex, :logger, :mix], &Application.load/1)

    # Start all applications with the makeup prefix
    for {app, _, _} <- Application.loaded_applications(),
        match?("makeup_" <> _, Atom.to_string(app)),
        do: Application.ensure_all_started(app)

    Supervisor.start_link([ExDoc.Refs], strategy: :one_for_one)
  end
end
