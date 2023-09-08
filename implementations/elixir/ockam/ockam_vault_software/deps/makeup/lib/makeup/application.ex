defmodule Makeup.Application do
  use Application

  alias Makeup.Registry

  def start(_type, _args) do
    Registry.create_name_registry()
    Registry.create_extension_registry()
    Supervisor.start_link([], strategy: :one_for_one)
  end
end
