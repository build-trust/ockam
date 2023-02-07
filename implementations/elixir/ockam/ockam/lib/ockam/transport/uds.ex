defmodule Ockam.Transport.UDS do
  use Supervisor

  alias Ockam.Router
  alias Ockam.Message
  alias Ockam.Wire
  alias Ockam.Transport.UDSAddress
  alias Ockam.Transport.UDS.{ConnectionSupervisor, Listener}

  def start(opts \\ []) do
    Router.set_message_handler(
      UDSAddress.type(),
      {__MODULE__, :handle_transport_message, []}
    )

    Supervisor.start_link(__MODULE__, opts)
  end

  @impl true
  def init(opts) do
    children = [
      ConnectionSupervisor,
      {Listener, opts}
    ]

    Supervisor.init(children, strategy: :rest_for_one)
  end

  def handle_transport_message(message) do
    with {:ok, destination, message} <-
           pick_destination_and_set_onward_route(message),
         {:ok, encoded_message} <- Wire.encode(message),
         {:ok, socket} <-
           :gen_tcp.connect({:local, destination}, 0, [:binary, active: true, reuseaddr: true]),
         :ok <- :gen_tcp.send(socket, encoded_message) do
      :ok
    else
      {:error, reason} ->
        {:error, reason}
    end
  end

  defp pick_destination_and_set_onward_route(message) do
    {dest_socket, onward_route} =
      message
      |> Message.onward_route()
      |> List.pop_at(0)

    with true <- UDSAddress.is_uds_address(dest_socket) do
      {:ok, dest_socket.value, %{message | onward_route: onward_route}}
    else
      false ->
        {:error, {:invalid_destination, dest_socket}}

      error ->
        error
    end
  end
end
