defmodule Ockam.Hub.Service.Provider do
  @moduledoc """
  Behaviour module and entrypoint to start Ockam.Hub services

  Provider behaviour implementations should provide a list of service names and be able to
  start service workers given names and arguments

  Provider can start all services configured in :ockam_hub => :services application environment
  with :ockam_hub => :providers provider implementations
  """

  require Logger

  @type child_spec :: Supervisor.child_spec() | {module(), term()} | module()

  ## TODO: maybe we need more than just a name here?
  @callback services() :: [atom()]

  @callback start_service(name :: atom(), args :: Keyword.t()) :: {:ok, address :: String.t()}
  @callback child_spec(name :: atom(), args :: Keyword.t()) :: child_spec()

  @spec start_configured_services() :: :ok | {:error, any()}
  def start_configured_services() do
    services = get_configured_services()

    ## TODO: validate config to fail with nice error
    start_result = start_services(services)

    start_errors =
      Enum.filter(
        start_result,
        fn
          {_, {:error, _}} -> true
          {_, {:ok, _}} -> false
        end
      )

    case Enum.empty?(start_errors) do
      true ->
        :ok

      false ->
        ## TODO: error formatting
        Logger.error("Errors starting services: #{inspect(start_errors)}")
        {:error, {:errors_starting_services, start_errors}}
    end
  end

  @spec configured_child_specs() :: {:ok, [child_spec()]} | {:error, any()}
  def configured_child_specs() do
    services = get_configured_services()
    spec_results = get_services_child_specs(services)

    {ok_results, errors} =
      Enum.split_with(spec_results, fn
        {:ok, _} -> true
        {:error, _} -> false
      end)

    case errors do
      [] ->
        {:ok, Enum.map(ok_results, fn {:ok, spec} -> spec end)}

      errors ->
        {:error, errors}
    end
  end

  @spec get_services_child_specs(Enum.t(), nil | map()) :: [
          {:ok, child_spec()} | {:error, any()}
        ]
  def get_services_child_specs(services_config, providers \\ nil) do
    service_providers_map = get_service_providers_map(providers)

    Enum.map(services_config, fn service_config ->
      do_get_service_child_spec(service_config, service_providers_map)
    end)
  end

  @spec get_service_child_spec(atom() | {atom(), list()}, nil | map()) ::
          {:ok, child_spec()} | {:error, any()}
  def get_service_child_spec(service_config, providers \\ nil)

  def get_service_child_spec(service_name, providers) when is_atom(service_name) do
    get_service_child_spec({service_name, []}, providers)
  end

  def get_service_child_spec({_, _} = service_config, providers) do
    service_providers_map = get_service_providers_map(providers)
    do_get_service_child_spec(service_config, service_providers_map)
  end

  @spec do_get_service_child_spec({atom(), list()}, map()) ::
          {:ok, child_spec()} | {:error, any()}
  def do_get_service_child_spec({service_name, service_args}, service_providers_map) do
    case Map.get(service_providers_map, service_name) do
      nil ->
        {:error, {:unknown_service, service_name}}

      provider_mod ->
        child_spec = provider_mod.child_spec(service_name, service_args)

        {:ok, Supervisor.child_spec(child_spec, id: service_name)}
    end
  end

  def start_services(services_config, providers \\ nil) do
    service_providers_map = get_service_providers_map(providers)

    Enum.map(services_config, fn service_config ->
      {service_config, do_start_service(service_config, service_providers_map)}
    end)
  end

  def start_service(service_name, providers \\ nil)

  def start_service(service_name, providers) when is_atom(service_name) do
    start_service({service_name, []}, providers)
  end

  def start_service(service_config, providers) do
    service_providers_map = get_service_providers_map(providers)

    do_start_service(service_config, service_providers_map)
  end

  def start_configured_service(service_name, extra_args \\ []) do
    services = get_configured_services()

    case Keyword.get(services, service_name) do
      nil ->
        {:error, :service_not_configured}

      default_args ->
        start_service({service_name, Keyword.merge(default_args, extra_args)})
    end
  end

  def do_start_service({service_name, service_args}, service_providers_map) do
    case Map.get(service_providers_map, service_name) do
      nil ->
        {:error, :unknown_service}

      provider_mod ->
        provider_mod.start_service(service_name, service_args)
    end
  end

  def get_service_providers_map(providers) do
    providers
    |> get_providers()
    |> Enum.flat_map(fn provider_mod ->
      Enum.map(provider_mod.services(), fn service -> {service, provider_mod} end)
    end)
    |> Map.new()
  end

  def get_providers(providers \\ nil)
  def get_providers(nil), do: Application.get_env(:ockam_hub, :service_providers)
  def get_providers(providers), do: providers

  def get_configured_services() do
    case Application.get_env(:ockam_hub, :services_config_source) do
      "json" ->
        parse_services_json(Application.get_env(:ockam_hub, :services_json))

      "file" ->
        parse_services_file(Application.get_env(:ockam_hub, :services_file))

      "list" ->
        parse_services_list(Application.get_env(:ockam_hub, :services_list, []))

      _other ->
        parse_services_config(Application.get_env(:ockam_hub, :services, []))
    end
  end

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

  def parse_services_list(nil) do
    []
  end

  def parse_services_list(services) do
    services
    |> String.split(",")
    |> Enum.map(fn service_name -> service_name |> String.trim() |> String.to_atom() end)
    |> parse_services_config()
  end

  def parse_services_json(nil) do
    []
  end

  def parse_services_json("") do
    []
  end

  def parse_services_json(json) do
    case Jason.decode(json, keys: :atoms) do
      {:ok, services} ->
        ## TODO: validate services
        services
        |> Enum.map(fn {service, args} -> {service, Enum.to_list(args)} end)
        |> Enum.to_list()

      {:error, err} ->
        raise("Unable to parse json services config: #{inspect(err)}")
    end
  end

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
