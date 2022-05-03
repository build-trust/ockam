defmodule Ockam.Services.Provider do
  @moduledoc """
  Behaviour module and entrypoint to start Ockam.Services services

  Provider behaviour implementations should provide a list of service names and be able to
  start service workers given names and arguments

  Provider can start all services configured in :ockam_services => :services application environment
  with :ockam_services => :providers provider implementations
  """

  require Logger

  @type child_spec :: Supervisor.child_spec() | {module(), term()} | module()

  @type service_config :: atom() | {atom(), list()}

  ## TODO: maybe we need more than just a name here?
  @callback services() :: [atom()]

  @callback child_spec(name :: atom(), args :: Keyword.t()) :: child_spec()

  def child_spec(args) do
    %{
      id: __MODULE__,
      start: {__MODULE__, :start_link, args}
    }
  end

  def start_link(providers \\ nil, services \\ nil)

  def start_link(providers, nil) do
    services = get_configured_services()
    start_link(providers, services)
  end

  def start_link(providers, services) do
    {child_specs, errors} = get_services_child_specs(services, providers)

    case errors do
      [] ->
        Supervisor.start_link(child_specs, name: __MODULE__, strategy: :one_for_one)

      errors ->
        {:error, errors}
    end
  end

  @spec get_services_child_specs(Enum.t(), nil | list()) :: {[child_spec()], [{:error, any()}]}
  def get_services_child_specs(services_config, providers \\ nil) do
    service_providers_map = get_service_providers_map(providers)

    spec_results =
      Enum.map(services_config, fn service_config ->
        get_service_child_spec(service_config, service_providers_map)
      end)

    {ok_results, errors} =
      Enum.split_with(spec_results, fn
        {:ok, _} -> true
        {:error, _} -> false
      end)

    child_specs = Enum.map(ok_results, fn {:ok, spec} -> spec end)

    {child_specs, errors}
  end

  @spec get_service_child_spec(service_config(), nil | list()) ::
          {:ok, child_spec()} | {:error, any()}
  def get_service_child_spec(service_config, providers \\ nil)

  def get_service_child_spec(service_name, providers) when is_atom(service_name) do
    get_service_child_spec({service_name, []}, providers)
  end

  def get_service_child_spec({service_name, service_args}, providers) do
    service_providers_map = get_service_providers_map(providers)

    case Map.get(service_providers_map, service_name) do
      nil ->
        {:error, {:unknown_service, service_name}}

      provider_mod ->
        child_spec =
          Supervisor.child_spec(provider_mod.child_spec(service_name, service_args),
            id: service_name
          )

        {:ok, child_spec}
    end
  end

  @spec start_service(service_config(), atom(), nil | list()) ::
          {:ok, pid()} | {:ok, pid(), any()} | {:error, any()}
  def start_service(service_config, supervisor, providers \\ nil) do
    case get_service_child_spec(service_config, providers) do
      {:ok, child_spec} ->
        Supervisor.start_child(supervisor, child_spec)

      {:error, reason} ->
        {:error, reason}
    end
  end

  @spec start_service(atom(), atom(), list()) ::
          {:ok, pid()} | {:ok, pid(), any()} | {:error, any()}
  def start_configured_service(service_name, supervisor, extra_args \\ []) do
    services = get_configured_services()

    case Keyword.get(services, service_name) do
      nil ->
        {:error, :service_not_configured}

      default_args ->
        start_service({service_name, Keyword.merge(default_args, extra_args)}, nil, supervisor)
    end
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

  @spec get_service_providers_map(nil | list()) :: list()
  def get_providers(providers \\ nil)
  def get_providers(nil), do: Application.get_env(:ockam_services, :service_providers)
  def get_providers(providers) when is_list(providers), do: providers

  @spec get_configured_services() :: [service_config()]
  def get_configured_services() do
    case Application.get_env(:ockam_services, :services_config_source) do
      "json" ->
        parse_services_json(Application.get_env(:ockam_services, :services_json))

      "file" ->
        parse_services_file(Application.get_env(:ockam_services, :services_file))

      "list" ->
        parse_services_list(Application.get_env(:ockam_services, :services_list, []))

      _other ->
        parse_services_config(Application.get_env(:ockam_services, :services, []))
    end
  end

  @spec parse_services_config(Enum.t()) :: [service_config()]
  def parse_services_config(services) do
    Enum.map(
      services,
      fn
        atom when is_atom(atom) -> {atom, []}
        {atom, args_map} when is_map(args_map) -> {atom, Map.to_list(args_map)}
        {_atom, _args} = config -> config
      end
    )
  end

  @doc false
  def parse_services_list(nil) do
    []
  end

  def parse_services_list(services) do
    services
    |> String.split(",")
    |> Enum.map(fn service_name -> service_name |> String.trim() |> String.to_atom() end)
    |> parse_services_config()
  end

  @doc false
  def parse_services_json(nil) do
    []
  end

  def parse_services_json("") do
    []
  end

  def parse_services_json(json) do
    case Poison.decode(json, keys: :atoms) do
      {:ok, services} ->
        ## TODO: validate services
        services
        |> Enum.map(fn {service, args} -> {service, Enum.to_list(args)} end)
        |> Enum.to_list()

      {:error, err} ->
        raise("Unable to parse json services config: #{inspect(err)}")
    end
  end

  @doc false
  def parse_services_file(nil) do
    raise("Services config file is not defined")
  end

  def parse_services_file(filename) do
    with true <- File.exists?(filename),
         {:ok, contents} <- File.read(filename),
         data <- String.trim(contents) do
      parse_services_json(data)
    else
      _other ->
        raise("Services file is not found: #{inspect(filename)}")
    end
  end
end
