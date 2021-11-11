defmodule Ockam.Session.Pluggable do
  @moduledoc """
  Routing session

  Initiator sends a first handshake message to the init_route on start
  and waits for a response from the responder.

  Responder receives the handshake, starts the data worker and sends a response

  After receiving the response, initiator starts the data worker.

  Utilizing pluggable handshake logic using `Ockam.Session.Handshake` behaviour
  """

  @doc """
  Shared function for data stage of the session

  State MUST have :data_state and :worker_mod keys
  """
  @spec handle_data_message(any(), %{:data_state => any(), :worker_mod => atom(), any() => any()}) ::
          {:ok, %{data_state: any()}} | {:error, any()} | {:stop, any(), %{data_state: any()}}
  def handle_data_message(message, state) do
    data_state = Map.fetch!(state, :data_state)
    worker_mod = Map.fetch!(state, :worker_mod)

    case worker_mod.handle_message(message, data_state) do
      {:ok, new_data_state} ->
        {:ok, Map.put(state, :data_state, new_data_state)}

      {:error, error} ->
        {:error, {:data_error, error}}

      {:stop, reason, new_data_state} ->
        {:stop, reason, Map.put(state, :data_state, new_data_state)}
    end
  end

  def initiator() do
    Ockam.Session.Pluggable.Initiator
  end

  def responder() do
    Ockam.Session.Pluggable.Responder
  end
end
