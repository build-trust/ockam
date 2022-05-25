defmodule Ockam.SecureChannel.KeyEstablishmentProtocol.XX.Protocol do
  @moduledoc false

  alias Ockam.Vault

  defstruct [
    # handle to a vault
    :vault,
    # identity keypair, reference in vault
    :s,
    # ephemeral keypair, reference in vault
    :e,
    # remote peer's identity public key
    :rs,
    # remote peer's ephemeral public key
    :re,
    # chaining key ck
    :ck,
    # encryption key k
    :k,
    # counter-based nonce n
    :n,
    # transcript hash, that hashes all the data that’s been sent and received.
    :h,
    # a prologue that is hashed into h
    :prologue,
    # payload for message1
    :m1_payload,
    # payload for message2
    :m2_payload,
    # payload for message2
    :m3_payload
  ]

  @default_prologue ""
  @default_m1_payload ""
  @default_m2_payload ""
  @default_m3_payload ""

  @protocol_name "Noise_XX_25519_AESGCM_SHA256"
  defmacro zero_padded_protocol_name do
    quote bind_quoted: binding() do
      padding_size = (32 - byte_size(@protocol_name)) * 8
      <<@protocol_name, 0::size(padding_size)>>
    end
  end

  def setup(options, data) do
    with {:ok, protocol_state} <- setup_vault(options, %__MODULE__{}),
         {:ok, protocol_state} <- setup_s(options, protocol_state),
         {:ok, protocol_state} <- setup_e(options, protocol_state),
         {:ok, protocol_state} <- setup_h(protocol_state),
         {:ok, protocol_state} <- setup_ck(protocol_state),
         {:ok, protocol_state} <- setup_prologe(options, protocol_state),
         {:ok, protocol_state} <- setup_message_payloads(options, protocol_state) do
      data = Map.put(data, :xx_key_establishment_state, protocol_state)
      {:ok, data}
    end
  end

  defp setup_vault(options, state) do
    case Keyword.get(options, :vault) do
      nil -> {:error, :vault_option_is_nil}
      vault -> {:ok, %{state | vault: vault}}
    end
  end

  defp setup_s(options, state) do
    case Keyword.get(options, :identity_keypair) do
      nil -> {:error, :identity_keypair_option_is_nil}
      %{private: _priv, public: _pub} = s -> {:ok, %{state | s: s}}
      vault_handle -> turn_vault_private_key_handle_to_keypair(:s, vault_handle, state)
    end
  end

  defp setup_e(options, state) do
    case Keyword.get(options, :ephemeral_keypair) do
      nil -> generate_e(state)
      %{private: _priv, public: _pub} = e -> {:ok, %{state | e: e}}
      vault_handle -> turn_vault_private_key_handle_to_keypair(:e, vault_handle, state)
    end
  end

  defp turn_vault_private_key_handle_to_keypair(s_or_e, handle, %{vault: vault} = state) do
    with {:ok, public_key} <- Vault.secret_publickey_get(vault, handle) do
      state = Map.put(state, s_or_e, %{private: handle, public: public_key})
      {:ok, state}
    end
  end

  defp generate_e(%{vault: vault} = state) do
    with {:ok, private_key} <- Vault.secret_generate(vault, type: :curve25519),
         {:ok, public_key} <- Vault.secret_publickey_get(vault, private_key) do
      e = %{private: private_key, public: public_key}
      {:ok, %{state | e: e}}
    else
      {:error, reason} -> {:error, {:could_not_setup_e, reason}}
    end
  end

  defp setup_h(state) do
    h = zero_padded_protocol_name()
    {:ok, %{state | h: h}}
  end

  defp setup_ck(%{vault: vault} = state) do
    case Vault.secret_import(vault, [type: :buffer], zero_padded_protocol_name()) do
      {:ok, ck} -> {:ok, %{state | ck: ck}}
      {:error, reason} -> {:error, {:could_not_setup_ck, reason}}
    end
  end

  defp setup_prologe(options, state) do
    prologue = Keyword.get(options, :prologue, @default_prologue)

    with {:ok, state} <- mix_hash(state, prologue) do
      {:ok, %{state | prologue: prologue}}
    end
  end

  defp setup_message_payloads(options, state) do
    state =
      state
      |> Map.put(:m1_payload, Keyword.get(options, :message1_payload, @default_m1_payload))
      |> Map.put(:m2_payload, Keyword.get(options, :message2_payload, @default_m2_payload))
      |> Map.put(:m3_payload, Keyword.get(options, :message3_payload, @default_m3_payload))

    {:ok, state}
  end

  def encode(message_name, %{xx_key_establishment_state: protocol_state} = data)
      when message_name in [:message1, :message2, :message3] do
    encoder = String.to_existing_atom("encode_" <> Atom.to_string(message_name))
    return_value = apply(__MODULE__, encoder, [protocol_state])

    case return_value do
      {:ok, encoded, protocol_state} ->
        ## TODO: optimise double encoding of binaries
        encoded_payload = :bare.encode(encoded, :data)
        {:ok, encoded_payload, %{data | xx_key_establishment_state: protocol_state}}

      {:error, reason} ->
        {:error, {:failed_to_encode, message_name, reason}}
    end
  end

  def decode(message_name, message_payload, %{xx_key_establishment_state: protocol_state} = data)
      when message_name in [:message1, :message2, :message3] do
    ## TODO: optimise double encoding of binaries
    {:ok, message, ""} = :bare.decode(message_payload, :data)
    decoder = String.to_existing_atom("decode_" <> Atom.to_string(message_name))
    return_value = apply(__MODULE__, decoder, [message, protocol_state])

    case return_value do
      {:ok, payload, protocol_state} ->
        {:ok, payload, %{data | xx_key_establishment_state: protocol_state}}

      {:error, reason} ->
        {:error, {:failed_to_decode, message_name, reason}}
    end
  end

  def encode_message1(%{e: e, m1_payload: payload} = state) do
    with {:ok, state} <- mix_hash(state, e.public),
         {:ok, state} <- mix_hash(state, payload) do
      {:ok, e.public <> payload, state}
    end
  end

  def encode_message2(%{e: e, s: s, re: re, m2_payload: payload} = state) do
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

  def encode_message3(%{s: s, re: re, m3_payload: payload} = state) do
    with {:ok, state, encrypted_s_and_tag} <- encrypt_and_hash(state, s.public),
         {:ok, shared_secret} <- dh(state, s, re),
         {:ok, state} <- mix_key(state, shared_secret),
         {:ok, state, encrypted_payload_and_tag} <- encrypt_and_hash(state, payload) do
      {:ok, encrypted_s_and_tag <> encrypted_payload_and_tag, state}
    end
  end

  def decode_message1(message, state) do
    with {:ok, re, payload} <- parse_message1(message),
         {:ok, state} <- mix_hash(state, re),
         {:ok, state} <- mix_hash(state, payload) do
      {:ok, payload, %{state | re: re}}
    end
  end

  def decode_message2(message, %{e: e} = state) do
    with {:ok, re, encrypted_rs_and_tag, encrypted_payload_and_tag} <- parse_message2(message),
         {:ok, state} <- mix_hash(state, re),
         {:ok, shared_secret} <- dh(state, e, re),
         {:ok, state} <- mix_key(state, shared_secret),
         {:ok, state, rs} <- decrypt_and_hash(state, encrypted_rs_and_tag),
         {:ok, shared_secret} <- dh(state, e, rs),
         {:ok, state} <- mix_key(state, shared_secret),
         {:ok, state, payload} <- decrypt_and_hash(state, encrypted_payload_and_tag) do
      {:ok, payload, %{state | re: re, rs: rs}}
    end
  end

  def decode_message3(message, %{e: e} = state) do
    with {:ok, encrypted_rs_and_tag, encrypted_payload_and_tag} <- parse_message3(message),
         {:ok, state, rs} <- decrypt_and_hash(state, encrypted_rs_and_tag),
         {:ok, shared_secret} <- dh(state, e, rs),
         {:ok, state} <- mix_key(state, shared_secret),
         {:ok, state, payload} <- decrypt_and_hash(state, encrypted_payload_and_tag) do
      {:ok, payload, %{state | rs: rs}}
    end
  end

  def parse_message1(<<re::32-bytes, payload::binary>>), do: {:ok, re, payload}
  def parse_message1(message), do: {:error, {:unexpected_structure, :message1, message}}

  def parse_message2(<<re::32-bytes, encrypted_rs_and_tag::48-bytes, rest::binary>>) do
    encrypted_payload_and_tag = rest
    {:ok, re, encrypted_rs_and_tag, encrypted_payload_and_tag}
  end

  def parse_message2(message), do: {:error, {:unexpected_structure, :message2, message}}

  def parse_message3(<<encrypted_rs_and_tag::48-bytes, encrypted_payload_and_tag::binary>>),
    do: {:ok, encrypted_rs_and_tag, encrypted_payload_and_tag}

  def parse_message3(message), do: {:error, {:unexpected_structure, :message3, message}}

  def mix_hash(%{vault: vault, h: h} = state, value) do
    case Vault.sha256(vault, h <> value) do
      {:ok, h} -> {:ok, %{state | h: h}}
      error -> {:error, {:could_not_mix_hash, {state, value, error}}}
    end
  end

  def mix_key(%{vault: vault, ck: ck} = state, input_key_material) do
    ck_attributes = %{type: :buffer, length: 32, persistence: :ephemeral}
    k_attributes = %{type: :aes, length: 32, persistence: :ephemeral}
    kdf_result = Vault.hkdf_sha256(vault, ck, input_key_material, [ck_attributes, k_attributes])

    with {:ok, [ck, k]} <- kdf_result do
      {:ok, %{state | n: 0, ck: ck, k: k}}
    end
  end

  def dh(%{vault: vault}, keypair, peer_public) do
    Vault.ecdh(vault, keypair.private, peer_public)
  end

  def encrypt_and_hash(%{vault: vault, k: k, n: n, h: h} = state, plaintext) do
    with {:ok, k} <- Vault.secret_export(vault, k),
         {:ok, k} <- Vault.secret_import(vault, [type: :aes], k),
         {:ok, ciphertext_and_tag} <- Vault.aead_aes_gcm_encrypt(vault, k, n, h, plaintext),
         :ok <- Vault.secret_destroy(vault, k),
         {:ok, state} <- mix_hash(state, ciphertext_and_tag) do
      {:ok, %{state | n: n + 1}, ciphertext_and_tag}
    end
  end

  def decrypt_and_hash(%{vault: vault, k: k, n: n, h: h} = state, ciphertext_and_tag) do
    with {:ok, k} <- Vault.secret_export(vault, k),
         {:ok, k} <- Vault.secret_import(vault, [type: :aes], k),
         {:ok, plaintext} <- Vault.aead_aes_gcm_decrypt(vault, k, n, h, ciphertext_and_tag),
         :ok <- Vault.secret_destroy(vault, k),
         {:ok, state} <- mix_hash(state, ciphertext_and_tag) do
      {:ok, %{state | n: n + 1}, plaintext}
    end
  end

  def split(%{xx_key_establishment_state: %{vault: vault, ck: ck, h: h}} = data) do
    k1_attributes = %{type: :aes, length: 32, persistence: :ephemeral}
    k2_attributes = %{type: :aes, length: 32, persistence: :ephemeral}

    with {:ok, [k1, k2]} <- Vault.hkdf_sha256(vault, ck, [k1_attributes, k2_attributes]) do
      {:ok, {k1, k2, h}, data}
    end
  end
end
