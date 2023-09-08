defmodule Dialyxir do
  @moduledoc false
  use Application
  alias Dialyxir.Output

  def start(_, _) do
    Output.info("""
    Warning: the `dialyxir` application's start function was called, which likely means you
    did not add the dependency with the `runtime: false` flag. This is not recommended because
    it will mean that unnecessary applications are started, and unnecessary applications are most
    likely being added to your PLT file, increasing build time.
    Please add `runtime: false` in your `mix.exs` dependency section e.g.:
    {:dialyxir, "~> 0.5", only: [:dev], runtime: false}
    """)

    Supervisor.start_link([], strategy: :one_for_one)
  end
end
