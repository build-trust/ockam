defmodule Ockam.Registry do
  @type service :: module()
  @type name :: term()

  def child_spec([]) do
    Registry.child_spec(keys: :unique, name: __MODULE__)
  end

  @doc "Lookup a registered service by name"
  @spec lookup(name()) :: nil | {pid(), service()}
  def lookup(name) do
    case Registry.lookup(__MODULE__, name) do
      [] ->
        nil

      [registered] ->
        registered
    end
  end

  @doc "Lookup registered instances of the given service"
  @spec lookup_by_service(service()) :: [{pid(), name()}]
  def lookup_by_service(service) do
    Registry.select(__MODULE__, [{{:"$1", :"$2", service}, [], [{:"$2", :"$1"}]}])
  end
end
