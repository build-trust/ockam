defmodule Ockam.Services do
  # credo:disable-for-this-file Credo.Check.Refactor.ModuleDependencies

  @moduledoc """
  Main application to run Ockam Services

  Supervisor runs ockam services and transports

  Starts all services configured in :ockam_services => :services application environment
  with :ockam_services => :providers provider implementations
  """

  use Application

  alias Ockam.Services.Provider
  alias Ockam.Services.Service

  require Logger

  @supervisor __MODULE__

  @doc false
  def start(_type, _args) do
    tcp_transport_options = Application.get_env(:ockam_services, :tcp_transport)
    udp_transport_options = Application.get_env(:ockam_services, :udp_transport)

    tcp_transport =
      case tcp_transport_options do
        nil -> []
        _options -> [{Ockam.Transport.TCP, tcp_transport_options}]
      end

    udp_transport =
      case udp_transport_options do
        nil -> []
        ## TODO: use same module format as TCP
        _options -> [{Ockam.Transport.UDP.Listener, udp_transport_options}]
      end

    with {:ok, services_child_specs} <- Provider.get_configured_services_child_specs() do
      children =
        tcp_transport ++
          udp_transport ++
          services_child_specs

      Supervisor.start_link(children, strategy: :one_for_one, name: @supervisor)
    end
  end

  @spec start_service(atom(), list(), atom()) ::
          [{:ok, pid()}] | [{:ok, pid(), any()}] | [{:error, any()}]
  def start_service(name, options \\ [], supervisor \\ @supervisor) do
    case Provider.get_service_child_specs({name, options}) do
      {:ok, child_specs} ->
        ## TODO: if there are multiple workers, group them in a separate supervisor
        Enum.map(child_specs, fn spec ->
          Supervisor.start_child(supervisor, spec)
        end)

      {:error, reason} ->
        [{:error, reason}]
    end
  end

  @spec stop_service(Ockam.Address.t(), atom()) :: :ok | {:error, :not_found}
  def stop_service(address, supervisor \\ @supervisor) do
    with {:ok, child_id} <- find_child_id(supervisor, address) do
      Supervisor.terminate_child(supervisor, child_id)
    end
  end

  defp find_child_id(supervisor, address) do
    ## TODO: easier way to find services child_ids by address
    ## Use addresses for child ids?
    case Ockam.Node.whereis(address) do
      nil ->
        {:error, :not_found}

      pid ->
        children = Supervisor.which_children(supervisor)

        case List.keyfind(children, pid, 1) do
          {id, ^pid, _type, _modules} -> {:ok, id}
          nil -> {:error, :not_found}
        end
    end
  end

  def list_services(supervisor \\ @supervisor) do
    supervisor
    |> Supervisor.which_children()
    |> Enum.map(fn child ->
      Service.from_child(child)
    end)
    |> Enum.flat_map(fn
      {:ok, service} -> [service]
      {:error, _reason} -> []
    end)
  end

  def get_service(id, supervisor \\ @supervisor) do
    children = Supervisor.which_children(supervisor)

    case List.keyfind(children, String.to_atom(id), 0) do
      {_id, _pid, _type, _modules} = child ->
        Service.from_child(child)

      _other ->
        {:error, :not_found}
    end
  end
end
