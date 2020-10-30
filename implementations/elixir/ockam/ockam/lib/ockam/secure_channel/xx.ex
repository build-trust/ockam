defmodule Ockam.SecureChannel.XX do
  @moduledoc """
  Defines the XX Key Agreement protocol.
  """

  alias Ockam.Vault

  defstruct [:role, :vault, :s, :e, :rs, :re, :ck, :k, :n, :h, :prologue]

  @protocol_name "Noise_XX_25519_AESGCM_SHA256"
  defmacro zero_padded_protocol_name do
    quote bind_quoted: binding() do
      padding_size = (32 - byte_size(@protocol_name)) * 8
      <<@protocol_name, 0::size(padding_size)>>
    end
  end

  def initialize(role, vault, s, e \\ nil) do
    state = %__MODULE__{role: role, vault: vault, s: s, e: e, prologue: ""}

    with {:ok, state} <- initialize_role(state),
         {:ok, state} <- initialize_vault(state),
         {:ok, state} <- initialize_s(state),
         {:ok, state} <- initialize_e(state),
         {:ok, state} <- initialize_h(state),
         {:ok, state} <- initialize_ck(state) do
      mix_hash(state, state.prologue)
    end
  end

  # initialize role - initiator or responder

  defp initialize_role(%{role: role} = state) when role in [:initiator, :responder] do
    {:ok, state}
  end

  defp initialize_role(%{role: role}),
    do: {:error, {:role_argument_has_an_unexpected_value, role}}

  # initialize vault

  defp initialize_vault(%{vault: nil}), do: {:error, :vault_argument_is_nil}
  defp initialize_vault(%{vault: _vault} = state), do: {:ok, state}

  # initialize identity keypair s

  defp initialize_s(%{s: %{private: _private_key, public: _public_key}} = state),
    do: {:ok, state}

  defp initialize_s(%{s: s}),
    do: {:error, {:s_argument_does_not_have_the_expected_structue, s}}

  # initialize ephemeral keypair e

  defp initialize_e(%{e: nil, vault: vault} = state) do
    secret_attributes = %{type: :curve25519, persistence: :ephemeral, purpose: :key_agreement}

    with {:ok, private_key} <- Vault.secret_generate(vault, secret_attributes),
         {:ok, public_key} <- Vault.secret_publickey_get(vault, private_key) do
      e = %{private: private_key, public: public_key}
      {:ok, %{state | e: e}}
    else
      {:error, reason} -> {:error, {:could_not_initialize_e, reason}}
    end
  end

  defp initialize_e(%{e: %{private: _private, public: _public}} = state), do: {:ok, state}

  defp initialize_e(%{e: e}), do: {:error, {:e_argument_does_not_have_the_expected_structue, e}}

  # initialize h

  defp initialize_h(state) do
    h = zero_padded_protocol_name()
    {:ok, %{state | h: h}}
  end

  # initialize ck

  defp initialize_ck(%{vault: vault} = state) do
    ck_attributes = %{type: :buffer, persistence: :ephemeral, purpose: :key_agreement}

    case Vault.secret_import(vault, ck_attributes, zero_padded_protocol_name()) do
      {:ok, ck} -> {:ok, %{state | ck: ck}}
      {:error, reason} -> {:error, {:could_not_initialize_ck, reason}}
    end
  end

  def encode_message_1(%__MODULE__{e: e} = state, payload) do
    with {:ok, state} <- mix_hash(state, e.public),
         {:ok, state} <- mix_hash(state, payload) do
      {:ok, e.public <> payload, state}
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
      {:ok, e.public <> encrypted_s_and_tag <> encrypted_payload_and_tag, state}
    end
  end

  def encode_message_3(%__MODULE__{s: s, re: re} = state, payload) do
    with {:ok, state, encrypted_s_and_tag} <- encrypt_and_hash(state, s.public),
         {:ok, shared_secret} <- dh(state, s, re),
         {:ok, state} <- mix_key(state, shared_secret),
         {:ok, state, encrypted_payload_and_tag} <- encrypt_and_hash(state, payload) do
      {:ok, encrypted_s_and_tag <> encrypted_payload_and_tag, state}
    end
  end

  def decode_message_1(state, message) do
    <<re::32-bytes, payload::binary>> = message

    with {:ok, state} <- mix_hash(state, re),
         {:ok, state} <- mix_hash(state, payload) do
      {:ok, payload, %{state | re: re}}
    end
  end

  def decode_message_2(%__MODULE__{e: e} = state, message) do
    <<re::32-bytes, encrypted_rs_and_tag::48-bytes, encrypted_payload_and_tag::binary>> = message

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
    <<encrypted_rs_and_tag::48-bytes, encrypted_payload_and_tag::binary>> = message

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
    with {:ok, [ck, k]} <- Vault.hkdf_sha256(vault, ck, input_key_material, 2) do
      # :ok <- Vault.set_secret_type(vault, k, :aes256)
      {:ok, %{state | n: 0, k: k, ck: ck}}
    end
  end

  def dh(%__MODULE__{vault: vault}, keypair, peer_public) do
    Vault.ecdh(vault, keypair.private, peer_public)
  end

  def encrypt_and_hash(%__MODULE__{vault: vault, k: k, n: n, h: h} = state, plaintext) do
    secret_attributes = %{type: :aes256, persistence: :ephemeral, purpose: :key_agreement}

    with {:ok, k} <- Vault.secret_export(vault, k),
         {:ok, k} <- Vault.secret_import(vault, secret_attributes, k),
         {:ok, ciphertext_and_tag} <- Vault.aead_aes_gcm_encrypt(vault, k, n, h, plaintext),
         :ok <- Vault.secret_destroy(vault, k),
         {:ok, state} <- mix_hash(state, ciphertext_and_tag) do
      {:ok, %{state | n: n + 1}, ciphertext_and_tag}
    end
  end

  def decrypt_and_hash(%__MODULE__{vault: vault, k: k, n: n, h: h} = state, ciphertext_and_tag) do
    secret_attributes = %{type: :aes256, persistence: :ephemeral, purpose: :key_agreement}

    with {:ok, k} <- Vault.secret_export(vault, k),
         {:ok, k} <- Vault.secret_import(vault, secret_attributes, k),
         {:ok, plaintext} <- Vault.aead_aes_gcm_decrypt(vault, k, n, h, ciphertext_and_tag),
         :ok <- Vault.secret_destroy(vault, k),
         {:ok, state} <- mix_hash(state, ciphertext_and_tag) do
      {:ok, %{state | n: n + 1}, plaintext}
    end
  end

  def split(%__MODULE__{vault: vault, ck: ck}) do
    Vault.hkdf_sha256(vault, ck, nil, 2)
  end
end
