defmodule Ockam.Services.Provider do
  @moduledoc """
  Behaviour module and helper functions to operate Ockam.Services services

  Provider behaviour implementations should provide a list of service names and be able to
  start service workers given names and arguments
  """

  require Logger

  @type child_spec :: Supervisor.child_spec() | {module(), term()} | module()

  @type service_config :: atom() | {atom(), list()}

  ## TODO: maybe we need more than just a name here?
  @callback services() :: [atom()]

  @callback child_spec(name :: atom(), args :: Keyword.t()) :: child_spec() | [child_spec()]

  @spec get_configured_services_child_specs() :: {:ok, [child_spec()]} | {:error, any()}
  def get_configured_services_child_specs() do
    services = get_configured_services()
    {child_specs, errors} = get_services_child_specs(services)

    case errors do
      [] ->
        {:ok, child_specs}

      errors ->
        {:error, {:cannot_load_child_specs, errors}}
    end
  end

  @spec get_services_child_specs(Enum.t(), nil | list()) :: {[child_spec()], [{:error, any()}]}
  def get_services_child_specs(services_config, providers \\ nil) do
    service_providers_map = get_service_providers_map(providers)

    spec_results =
      Enum.map(services_config, fn service_config ->
        get_service_child_specs(service_config, service_providers_map)
      end)

    {ok_results, errors} =
      Enum.split_with(spec_results, fn
        {:ok, _} -> true
        {:error, _} -> false
      end)

    child_specs = Enum.flat_map(ok_results, fn {:ok, specs} -> specs end)

    {child_specs, errors}
  end

  @spec get_service_child_specs(service_config(), nil | list()) ::
          {:ok, [child_spec()]} | {:error, any()}
  def get_service_child_specs(service_config, providers \\ nil)

  def get_service_child_specs(service_name, providers) when is_atom(service_name) do
    get_service_child_specs({service_name, []}, providers)
  end

  def get_service_child_specs({service_name, service_args}, providers) do
    case get_named_service_child_specs(service_name, service_args, providers) do
      {:ok, specs} ->
        {:ok, specs}

      {:error, {:unknown_service, service_name}} ->
        case is_service_module?(service_name) do
          true ->
            {:ok, [module_child_spec(service_name, service_args)]}

          false ->
            {:error, {:unknown_service, service_name}}
        end
    end
  end

  defp is_service_module?(service_name) do
    case Code.ensure_loaded(service_name) do
      {:module, _module} ->
        function_exported?(service_name, :child_spec, 1)

      {:error, _reason} ->
        false
    end
  end

  defp module_child_spec(service_name, service_args) do
    id = Keyword.get(service_args, :id, service_name)

    Supervisor.child_spec({service_name, service_args}, id: id)
  end

  defp get_named_service_child_specs(service_name, service_args, providers) do
    service_providers_map = get_service_providers_map(providers)

    case Map.get(service_providers_map, service_name) do
      nil ->
        {:error, {:unknown_service, service_name}}

      provider_mod ->
        try do
          case provider_mod.child_spec(service_name, service_args) do
            multiple_specs when is_list(multiple_specs) ->
              {:ok, fix_child_ids(multiple_specs)}

            %{id: _id} = single_spec_map ->
              {:ok, [single_spec_map]}

            single_spec ->
              ## Temporary measure to allow multiple services of the same type.
              ## TODO: redo the way address, service name and id are related
              id =
                case Keyword.fetch(service_args, :address) do
                  {:ok, address} -> String.to_atom(address)
                  :error -> service_name
                end

              {:ok, [Supervisor.child_spec(single_spec, id: id)]}
          end
        rescue
          err ->
            {:error, err}
        end
    end
  end

  ## Temporary measure to allow multiple children of the same type.
  ## TODO: redo the way address, service name and id are related
  defp fix_child_ids(specs) do
    Enum.map(
      specs,
      fn
        %{id: _id} = spec ->
          spec

        {mod, args} = spec ->
          case Keyword.fetch(args, :address) do
            {:ok, address} ->
              id = String.to_atom(address)
              Supervisor.child_spec(spec, id: id)

            :error ->
              {mod, args}
          end

        other ->
          other
      end
    )
  end

  @spec get_service_providers_map(nil | list()) :: map()
  def get_service_providers_map(providers) when is_list(providers) or providers == nil do
    providers
    |> get_providers()
    |> Enum.flat_map(fn provider_mod ->
      Enum.map(provider_mod.services(), fn service -> {service, provider_mod} end)
    end)
    |> Map.new()
  end

  def get_service_providers_map(providers_map) when is_map(providers_map) do
    providers_map
  end

  @spec get_providers(nil | list()) :: list()
  def get_providers(providers \\ nil)
  def get_providers(nil), do: Application.get_env(:ockam_services, :service_providers, [])
  def get_providers(providers) when is_list(providers), do: providers

  @spec get_configured_services() :: [service_config()]
  def get_configured_services() do
    parse_services_config(Application.get_env(:ockam_services, :services, []))
  end

  @spec parse_services_config(Enum.t()) :: [service_config()]
  def parse_services_config(services) do
    Enum.map(
      services,
      fn
        atom when is_atom(atom) -> {atom, []}
        {atom, args_map} when is_map(args_map) -> {atom, Map.to_list(args_map)}
        {atom, args} = config when is_atom(atom) and is_list(args) -> config
      end
    )
  end
end
