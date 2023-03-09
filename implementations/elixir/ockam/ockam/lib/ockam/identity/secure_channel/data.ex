defmodule Ockam.Identity.SecureChannel.Data do
  @moduledoc """
  Data stage for identity secure channel

  Options:
  - peer_address - address of the channel peer
  - encryption_channel - address of local end of encryption channel
  - identity - own identity
  - contact_id - ID of remote identity
  - contact - remote identity
  """
  use Ockam.AsymmetricWorker

  alias Ockam.Identity.SecureChannel.ServiceMessage

  alias Ockam.Message
  alias Ockam.Router

  @impl true
  def inner_setup(options, state) do
    ## TODO: access control to only get secure channel message on the inner address
    peer_address = Keyword.fetch!(options, :peer_address)
    encryption_channel = Keyword.fetch!(options, :encryption_channel)
    identity = Keyword.fetch!(options, :identity)
    contact_id = Keyword.fetch!(options, :contact_id)
    contact = Keyword.fetch!(options, :contact)
    additional_metadata = Keyword.get(options, :additional_metadata, %{})

    inner_address = Map.fetch!(state, :inner_address)

    ## Outer address authorization
    state =
      case Keyword.fetch(options, :authorization) do
        {:ok, authorization} ->
          Ockam.Worker.update_authorization_state(state, authorization)

        :error ->
          state
      end

    ## Inner address authorization
    state =
      Ockam.Worker.update_authorization_state(state, inner_address, [
        :from_secure_channel,
        {:from_addresses, [:message, [encryption_channel]]}
      ])

    {:ok,
     Map.merge(
       state,
       %{
         peer_address: peer_address,
         encryption_channel: encryption_channel,
         identity: identity,
         contact_id: contact_id,
         contact: contact,
         additional_metadata: additional_metadata
       }
     )}
  end

  @impl true
  def handle_inner_message(
        message,
        state
      ) do
    with [_me | onward_route] <- Message.onward_route(message),
         [_channel | _return_route] <- Message.return_route(message) do
      ## Assertion. This should be checked by authorization
      %{channel: :secure_channel, source: :channel} = Message.local_metadata(message)

      case onward_route do
        [] ->
          ## Message is directed to the secure channel worker
          handle_service_message(message, state)

        _route ->
          handle_data_message(message, state)
      end
    else
      _other ->
        {:error, {:invalid_inner_message, message}}
    end
  end

  defp handle_service_message(message, state) do
    payload = Message.payload(message)

    case ServiceMessage.decode_strict(payload) do
      {:ok, %ServiceMessage{command: :disconnect}} ->
        Logger.info("Secure channel disconnected")
        ## TODO: should we use reason other than normal here?
        Ockam.Node.stop(Map.fetch!(state, :encryption_channel))
        {:stop, :normal, state}

      {:ok, other} ->
        Logger.warn("Unsupported service message: #{inspect(other)}")
        {:ok, state}

      {:error, _reason} ->
        ## It's not a service message, handle as data message routed nowhere
        ## TODO: should we just error here instead?
        handle_data_message(message, state)
    end
  end

  defp handle_data_message(
         message,
         %{
           address: address,
           contact_id: contact_id,
           contact: contact,
           additional_metadata: additional_metadata
         } = state
       ) do
    [_me | onward_route] = Message.onward_route(message)
    [_channel | return_route] = Message.return_route(message)
    payload = Message.payload(message)

    metadata =
      Map.merge(additional_metadata, %{
        channel: :identity_secure_channel,
        source: :channel,
        identity_id: contact_id,
        identity: contact
      })

    forwarded_message =
      %Message{
        payload: payload,
        onward_route: onward_route,
        return_route: [address | return_route]
      }
      |> Message.set_local_metadata(metadata)

    Router.route(forwarded_message)
    {:ok, state}
  end

  @impl true
  def handle_outer_message(
        message,
        %{encryption_channel: channel, peer_address: peer} = state
      ) do
    case Message.onward_route(message) do
      [_me | onward_route] ->
        forwarded_message =
          message
          |> Message.set_onward_route([channel, peer | onward_route])
          |> Message.put_local_metadata(:from_pid, self())

        Router.route(forwarded_message)
        {:ok, state}

      _other ->
        {:error, {:invalid_outer_message, message}}
    end
  end

  @impl true
  def handle_call(:get_remote_identity, _form, state) do
    contact = Map.fetch!(state, :contact)
    {:reply, contact, state}
  end

  def handle_call(:get_remote_identity_id, _form, state) do
    contact_id = Map.fetch!(state, :contact_id)
    {:reply, contact_id, state}
  end

  def handle_call(:disconnect, _from, %{encryption_channel: channel, peer_address: peer} = state) do
    disconnect_message = %Message{
      payload: ServiceMessage.encode!(%ServiceMessage{command: :disconnect}),
      onward_route: [channel, peer],
      return_route: [],
      local_metadata: %{from_pid: self()}
    }

    ## Technically it should be fine to send route first and then stop and the message
    ## should be processed before the exit signal.
    Router.route(disconnect_message)
    Ockam.Node.stop(channel)
    {:stop, :normal, :ok, state}
  end
end
