defmodule Ockam.Channel.XX do
  @moduledoc """
  Defines the XX Key Agreement protocol.
  """

  alias Ockam.Message
  alias Ockam.Router
  alias Ockam.Vault

  defstruct [:vault, :e, :s, :rs, :re, :ck, :k, :n, :h, :prologue]

  @protocol_name "Noise_XX_25519_AESGCM_SHA256"
  defmacro zero_padded_protocol_name do
    quote bind_quoted: binding() do
      padding_size = (32 - byte_size(@protocol_name)) * 8
      <<@protocol_name, 0::size(padding_size)>>
    end
  end

  # @public_key_length 32
  # @tag_length 16
  # @encrypted_public_key_and_tag_length @public_key_length + @tag_length

  def init(%{role: role, vault: vault, s: s} = data) do
    prologue = Map.get(data, :prologue, "")

    e =
      Map.get_lazy(data, :e, fn ->
        with {:ok, private_key} <- Vault.generate_secret(vault, type: :curve25519_private),
             {:ok, public_key} <- Vault.get_public_key(vault, private_key) do
          %{private: private_key, public: public_key}
        end
      end)

    h = zero_padded_protocol_name()
    state = %__MODULE__{vault: vault, e: e, s: s, h: h, prologue: prologue}

    with {:ok, ck} <- Vault.import_secret(vault, zero_padded_protocol_name()),
         {:ok, state} <- mix_hash(%{state | ck: ck}, prologue) do
      data = Map.put(data, :key_establisher_state, state)

      case role do
        :initiator -> {:ok, {:key_establishment, :initiator, :awaiting_trigger}, data}
        :responder -> {:ok, {:key_establishment, :responder, :awaing_m1}, data}
      end
    end
  end

  def handle({:trigger, onward_route}, {:key_establishment, :initiator, :awaiting_trigger}, data) do
    %{key_establisher_state: state} = data
    {:ok, m1, state} = encode_message_1(state, "")
    Router.route(%Message{payload: m1, onward_route: onward_route, return_route: [data.ciphertext_address]})
    data = %{data | key_establisher_state: state}
    {:next_state, {:key_establishment, :initiator, :awaing_m2}, data}
  end

  def handle({:ciphertext, message}, {:key_establishment, :initiator, :awaing_m2}, data) do
    %Message{payload: m2, return_route: return_route} = message
    %{key_establisher_state: state} = data

    {:ok, "", state} = decode_message_2(state, m2)
    {:ok, m3, state} = encode_message_3(state, "")
    {:ok, [k1, k2]} = split(state)
    :ok = Vault.set_secret_type(state.vault, k1, :aes256)
    :ok = Vault.set_secret_type(state.vault, k2, :aes256)

    data =
      Map.put(data, :data_state, %{
        vault: state.vault,
        route_to_peer: return_route,
        decrypt: {k1, 0},
        encrypt: {k2, 0},
        h: state.h
      })

    Router.route(%Message{payload: m3, onward_route: return_route, return_route: [data.ciphertext_address]})
    {:next_state, :data, %{data | key_establisher_state: state}}
  end

  # responder states

  def handle({:ciphertext, message}, {:key_establishment, :responder, :awaing_m1}, data) do
    %Message{payload: m1, return_route: return_route} = message
    %{key_establisher_state: state} = data

    {:ok, "", state} = decode_message_1(state, m1)
    {:ok, m2, state} = encode_message_2(state, "")

    Router.route(%Message{payload: m2, onward_route: return_route, return_route: [data.ciphertext_address]})

    {:next_state, {:key_establishment, :responder, :awaing_m3},
     %{data | key_establisher_state: state}}
  end

  def handle({:ciphertext, message}, {:key_establishment, :responder, :awaing_m3}, data) do
    %Message{payload: m3, return_route: return_route} = message
    %{key_establisher_state: state} = data

    {:ok, "", state} = decode_message_3(state, m3)
    {:ok, [k1, k2]} = split(state)
    :ok = Vault.set_secret_type(state.vault, k1, :aes256)
    :ok = Vault.set_secret_type(state.vault, k2, :aes256)

    data =
      Map.put(data, :data_state, %{
        vault: state.vault,
        route_to_peer: return_route,
        encrypt: {k1, 0},
        decrypt: {k2, 0},
        h: state.h
      })

    {:next_state, :data, %{data | key_establisher_state: state}}
  end

  def encode_message_1(%__MODULE__{e: e} = state, payload) do
    with {:ok, state} <- mix_hash(state, e.public),
         {:ok, state} <- mix_hash(state, payload) do
      {:ok, <<3::8>> <> e.public <> payload, state}
    end
  end

  def encode_message_2(%__MODULE__{e: e, s: s, re: re} = state, payload) do
    with {:ok, state} <- mix_hash(state, e.public),
         {:ok, shared_secret} <- dh(state, e, re),
         {:ok, state} <- mix_key(state, shared_secret),
         {:ok, state, encrypted_s_and_tag} <- encrypt_and_hash(state, s.public),
         {:ok, shared_secret} <- dh(state, s, re),
         {:ok, state} <- mix_key(state, shared_secret),
         {:ok, state, encrypted_payload_and_tag} <- encrypt_and_hash(state, payload) do
      {:ok, <<4::8>> <> e.public <> encrypted_s_and_tag <> encrypted_payload_and_tag, state}
    end
  end

  def encode_message_3(%__MODULE__{s: s, re: re} = state, payload) do
    with {:ok, state, encrypted_s_and_tag} <- encrypt_and_hash(state, s.public),
         {:ok, shared_secret} <- dh(state, s, re),
         {:ok, state} <- mix_key(state, shared_secret),
         {:ok, state, encrypted_payload_and_tag} <- encrypt_and_hash(state, payload) do
      {:ok, <<5::8>> <> encrypted_s_and_tag <> encrypted_payload_and_tag, state}
    end
  end

  def decode_message_1(state, message) do
    <<3::8, re::32-bytes, payload::binary>> = message

    with {:ok, state} <- mix_hash(state, re),
         {:ok, state} <- mix_hash(state, payload) do
      {:ok, payload, %{state | re: re}}
    end
  end

  def decode_message_2(%__MODULE__{e: e} = state, message) do
    <<4::8, re::32-bytes, encrypted_rs_and_tag::48-bytes, encrypted_payload_and_tag::binary>> = message

    with {:ok, state} <- mix_hash(state, re),
         {:ok, shared_secret} <- dh(state, e, re),
         {:ok, state} <- mix_key(state, shared_secret),
         {:ok, state, rs} <- decrypt_and_hash(state, encrypted_rs_and_tag),
         {:ok, shared_secret} <- dh(state, e, rs),
         {:ok, state} <- mix_key(state, shared_secret),
         {:ok, state, payload} <- decrypt_and_hash(state, encrypted_payload_and_tag) do
      {:ok, payload, %{state | re: re, rs: rs}}
    end
  end

  def decode_message_3(%__MODULE__{e: e} = state, message) do
    <<5::8, encrypted_rs_and_tag::48-bytes, encrypted_payload_and_tag::binary>> = message

    with {:ok, state, rs} <- decrypt_and_hash(state, encrypted_rs_and_tag),
         {:ok, shared_secret} <- dh(state, e, rs),
         {:ok, state} <- mix_key(state, shared_secret),
         {:ok, state, payload} <- decrypt_and_hash(state, encrypted_payload_and_tag) do
      {:ok, payload, %{state | rs: rs}}
    end
  end

  def mix_hash(%__MODULE__{vault: vault, h: h} = state, data) do
    case Vault.sha256(vault, h <> data) do
      {:ok, h} -> {:ok, %{state | h: h}}
      error -> {:error, error}
    end
  end

  def mix_key(%__MODULE__{vault: vault, ck: ck} = state, input_key_material) do
    with {:ok, [ck, k]} <- Vault.hkdf_sha256(vault, ck, input_key_material, 2),
         :ok <- Vault.set_secret_type(vault, k, :aes256) do
      {:ok, %{state | n: 0, k: k, ck: ck}}
    end
  end

  def dh(%__MODULE__{vault: vault}, keypair, peer_public) do
    Vault.ecdh(vault, keypair.private, peer_public)
  end

  def encrypt_and_hash(%__MODULE__{vault: vault, k: k, n: n, h: h} = state, plaintext) do
    with {:ok, ciphertext_and_tag} <- Vault.encrypt(vault, k, n, h, plaintext),
         {:ok, state} <- mix_hash(state, ciphertext_and_tag) do
      {:ok, %{state | n: n + 1}, ciphertext_and_tag}
    end
  end

  def decrypt_and_hash(%__MODULE__{vault: vault, k: k, n: n, h: h} = state, ciphertext_and_tag) do
    with {:ok, plaintext} <- Vault.decrypt(vault, k, n, h, ciphertext_and_tag),
         {:ok, state} <- mix_hash(state, ciphertext_and_tag) do
      {:ok, %{state | n: n + 1}, plaintext}
    end
  end

  def split(%__MODULE__{vault: vault, ck: ck}) do
    Vault.hkdf_sha256(vault, ck, nil, 2)
  end
end
