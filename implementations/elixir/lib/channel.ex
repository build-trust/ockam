defmodule Ockam.Channel do
  @moduledoc """
  An implementation of secure channels via the Noise protocol

  See an overview of the Noise handshake [here](https://noiseprotocol.org/noise.html#overview-of-handshake-state-machine)
  """
  require Logger

  alias Ockam.Transport
  alias Ockam.Channel.Handshake
  alias Ockam.Channel.Protocol
  alias Ockam.Channel.CipherState
  alias Ockam.Router.Protocol.Encoding
  alias Ockam.Router.Protocol.Message.Envelope
  alias Ockam.Router.Protocol.Message.Payload
  alias Ockam.Vault

  defstruct [:vault, :rx, :tx, :hash, :state]

  @type t :: %__MODULE__{
          vault: Vault.t(),
          rx: CipherState.t(),
          tx: CipherState.t(),
          hash: binary(),
          state: Ockam.Noise.Handshake.t()
        }
  @type role :: :initiator | :responder
  @type reason :: term()
  @type step_data :: {:send, payload :: binary()} | {:received, encrypted :: binary()}

  @roles [:initiator, :responder]

  @doc """
  Encrypt a message to be sent over the given channel
  """
  def encrypt(%__MODULE__{vault: vault, tx: tx} = chan, payload) do
    {:ok, new_tx, ciphertext} = CipherState.encrypt(tx, vault, "", payload)
    {:ok, %__MODULE__{chan | tx: new_tx}, ciphertext}
  end

  @doc """
  Decrypt a message received over the given channel
  """
  def decrypt(%__MODULE__{vault: vault, rx: rx} = chan, payload) do
    with {:ok, new_rx, plaintext} <- CipherState.decrypt(rx, vault, "", payload) do
      {:ok, %__MODULE__{chan | rx: new_rx}, plaintext}
    end
  end

  @doc """
  Start a handshake
  """
  @spec handshake(Vault.t(), role(), map()) ::
          {:ok, Handshake.t()} | {:error, {module(), reason()}}
  def handshake(vault, role, options)

  def handshake(%Vault{} = vault, role, options) when role in @roles and is_map(options) do
    prologue = Map.get(options, :prologue, "")

    protocol =
      case Map.get(options, :protocol) do
        name when is_binary(name) ->
          with {:ok, p} <- Protocol.from_name(name) do
            p
          else
            err ->
              throw(err)
          end

        %Protocol{} = p ->
          p
      end

    s = Map.get(options, :s)
    e = Map.get(options, :e)
    rs = Map.get(options, :rs)
    re = Map.get(options, :re)

    Handshake.init(vault, protocol, role, prologue, {s, e, rs, re})
  catch
    :throw, err ->
      err
  end

  def handshake(%Vault{}, role, _options) when role not in @roles,
    do: {:error, {__MODULE__, {:invalid_role, role}}}

  def handshake(%Vault{}, _role, _options),
    do: {:error, {__MODULE__, {:invalid_options, :expected_map}}}

  @doc """
  Step the handshake state machine forward one step
  """
  @spec step_handshake(Handshake.t(), step_data()) ::
          {:ok, :send, binary(), Handshake.t()}
          | {:ok, :received, binary(), Handshake.t()}
          | {:ok, :done, t()}
          | {:error, {__MODULE__, reason()}}
  def step_handshake(handshake, data)

  def step_handshake(%Handshake{} = handshake, data) do
    next = Handshake.next_message(handshake)
    step_handshake(next, data, handshake)
  end

  defp step_handshake(:in, {:received, encrypted}, handshake) do
    with {:ok, hs, msg} <- Handshake.read_message(handshake, encrypted) do
      {:ok, :received, msg, hs}
    end
  end

  defp step_handshake(:out, {:send, payload}, handshake) do
    with {:ok, hs, msg} <- Handshake.write_message(handshake, payload) do
      {:ok, :send, msg, hs}
    end
  end

  defp step_handshake(:done, :done, handshake) do
    with {:ok, chan} <- Handshake.finalize(handshake) do
      {:ok, :done, chan}
    end
  end

  defp step_handshake(next, data, _handshake) do
    {:error, {__MODULE__, {:invalid_step, {:expected, next}, {:got, data}}}}
  end

  @doc """
  Perform a Noise handshake to secure a channel, using the provided transport
  """
  @spec negotiate_secure_channel(Handshake.t(), Transport.t(), map()) ::
          {:ok, t(), Transport.t()} | {:error, {__MODULE__, term()}}
  @spec negotiate_secure_channel(Vault.t(), role(), Transport.t(), map()) ::
          {:ok, t(), Transport.t()} | {:error, {__MODULE__, term()}}
  def negotiate_secure_channel(vault, role, transport, options)

  def negotiate_secure_channel(%Vault{} = vault, role, transport, options) when role in @roles do
    with {:ok, handshake} <- handshake(vault, role, options) do
      timeout = Map.get(options, :timeout, :infinity)
      do_negotiate_secure_channel(handshake, transport, timeout)
    end
  end

  def negotiate_secure_channel(%Handshake{} = handshake, transport, options)
      when is_map(options) do
    timeout = Map.get(options, :timeout, :infinity)
    do_negotiate_secure_channel(handshake, transport, timeout)
  end

  defp do_negotiate_secure_channel(%Handshake{} = handshake, transport, timeout) do
    next = Handshake.next_message(handshake)
    Logger.debug("[#{inspect(handshake.role)}] Transitioning handshake to #{inspect(next)}")
    do_negotiate_secure_channel(next, handshake, transport, timeout)
  end

  defp do_negotiate_secure_channel(:in, handshake, transport, timeout) do
    Logger.debug("[#{inspect(handshake.role)}] Awaiting handshake message")

    with {:ok, data, transport} <- Transport.recv(transport, timeout: timeout),
         {:ok, %Envelope{body: %Payload{data: data}}, _rest} <- Encoding.decode(data),
         {:ok, hs, _msg} <- Handshake.read_message(handshake, data) do
      do_negotiate_secure_channel(hs, transport, timeout)
    else
      {:ok, message, _} ->
        {:error, {:unexpected_message, message}}

      {:error, _} = err ->
        err
    end
  end

  defp do_negotiate_secure_channel(:out, handshake, transport, timeout) do
    Logger.debug("[#{inspect(handshake.role)}] Sending handshake message")

    with {:ok, hs, msg} <- Handshake.write_message(handshake, ""),
         {:ok, encoded} <- Encoding.encode(%Payload{data: msg}),
         {:ok, transport} <- Transport.send(transport, encoded) do
      do_negotiate_secure_channel(hs, transport, timeout)
    end
  end

  defp do_negotiate_secure_channel(:done, handshake, transport, _timeout) do
    Logger.debug("[#{inspect(handshake.role)}] Finalizing handshake")

    with {:ok, chan} <- Handshake.finalize(handshake) do
      {:ok, chan, transport}
    end
  end
end
