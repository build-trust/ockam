defmodule Ockam.Vault do
  @moduledoc """
  """

  alias Ockam.Vault.NIF

  defmodule Secret do
    @moduledoc false
    defstruct [:reference]

    @opaque t :: %__MODULE__{}
  end

  defmodule SecretAttributes do
    @moduledoc false
    defstruct ty: :buffer, length: 0, persistence: :ephemeral, purpose: :key_agreement
  end

  defstruct [:reference]

  @opaque t :: %__MODULE__{}

  @spec create(Keyword.t()) :: {:ok, t} | {:error, term}
  def create(options \\ []) when is_list(options) do
    with {:ok, reference} <- NIF.make_vault(), do: {:ok, %__MODULE__{reference: reference}}
  end

  @spec sha256(t, binary) :: {:ok, binary} | {:error, term}
  def sha256(%__MODULE__{reference: vault_ref}, data) when is_binary(data),
    do: NIF.sha256(vault_ref, data)

  def generate_secret(%__MODULE__{reference: vault_ref}, attributes \\ []) do
    {type, attributes} = Keyword.pop(attributes, :type)
    attributes = if type, do: Keyword.put(attributes, :ty, type), else: attributes
    attributes = struct(SecretAttributes, attributes)

    with {:ok, secret_ref} <- NIF.generate_secret(vault_ref, attributes) do
      {:ok, %Secret{reference: secret_ref}}
    end
  end

  def generate_curve25519_keypair(vault) do
    with {:ok, private_key} <- generate_secret(vault, type: :curve25519_private),
         {:ok, public_key} <- get_public_key(vault, private_key) do
      {:ok, %{private: private_key, public: public_key}}
    end
  end

  def import_secret(%__MODULE__{reference: vault_ref}, secret, attributes \\ []) do
    {type, attributes} = Keyword.pop(attributes, :type)
    attributes = if type, do: Keyword.put(attributes, :ty, type), else: attributes
    attributes = struct(SecretAttributes, attributes)
    attributes = Map.put(attributes, :length, byte_size(secret))

    with {:ok, secret_ref} <- NIF.import_secret(vault_ref, secret, attributes) do
      {:ok, %Secret{reference: secret_ref}}
    end
  end

  def export_secret(%__MODULE__{reference: vault_ref}, %Secret{reference: secret_ref}),
    do: NIF.export_secret(vault_ref, secret_ref)

  def get_secret_attributes(%__MODULE__{reference: vault_ref}, %Secret{reference: secret_ref}) do
    with {:ok, attributes} <- NIF.get_secret_attributes(vault_ref, secret_ref) do
      {type, attributes} = attributes |> Map.from_struct() |> Map.to_list() |> Keyword.pop(:ty)
      if type, do: Keyword.put(attributes, :type, type), else: attributes
    end
  end

  def set_secret_type(%__MODULE__{reference: vault_ref}, %Secret{reference: secret_ref}, type),
    do: NIF.set_secret_type(vault_ref, secret_ref, type)

  @spec get_public_key(t, Secret.t()) :: {:ok, binary()} | {:error, term}
  def get_public_key(%__MODULE__{reference: vault_ref}, %Secret{reference: secret_ref}),
    do: NIF.get_public_key(vault_ref, secret_ref)

  def ecdh(%__MODULE__{reference: vault_ref}, %Secret{reference: private_key_ref}, peer_pubkey) do
    with {:ok, secret_ref} <- NIF.ecdh(vault_ref, private_key_ref, peer_pubkey) do
      {:ok, %Secret{reference: secret_ref}}
    end
  end

  def hkdf_sha256(
        %__MODULE__{reference: vault_ref},
        %Secret{reference: salt_ref},
        nil,
        num_outputs
      ) do
    with {:ok, outputs} <- NIF.hkdf_sha256(vault_ref, salt_ref, nil, num_outputs) do
      {:ok, Enum.map(outputs, fn x -> %Secret{reference: x} end)}
    end
  end

  def hkdf_sha256(
        %__MODULE__{reference: vault_ref},
        %Secret{reference: salt_ref},
        %Secret{reference: ikm_ref},
        num_outputs
      ) do
    with {:ok, outputs} <- NIF.hkdf_sha256(vault_ref, salt_ref, ikm_ref, num_outputs) do
      {:ok, Enum.map(outputs, fn x -> %Secret{reference: x} end)}
    end
  end

  def encrypt(
        %__MODULE__{reference: vault_ref},
        %Secret{reference: key_ref},
        nonce,
        aad,
        plaintext
      )
      when is_integer(nonce) do
    NIF.aead_aes_gcm_encrypt(vault_ref, key_ref, nonce, aad, plaintext)
  end

  def decrypt(
        %__MODULE__{reference: vault_ref},
        %Secret{reference: key_ref},
        nonce,
        aad,
        ciphertext_and_tag
      )
      when is_integer(nonce) do
    NIF.aead_aes_gcm_decrypt(vault_ref, key_ref, nonce, aad, ciphertext_and_tag)
  end

  defmodule NIF do
    @moduledoc false

    use Rustler, otp_app: :ockam, crate: :ockam_nif

    def make_vault, do: exit(:nif_not_loaded)
    def random(_vault), do: exit(:nif_not_loaded)
    def sha256(_vault, _data), do: exit(:nif_not_loaded)
    def generate_secret(_vault, _attrs), do: exit(:nif_not_loaded)
    def import_secret(_vault, _data, _attrs), do: exit(:nif_not_loaded)
    def export_secret(_vault, _secret), do: exit(:nif_not_loaded)
    def get_secret_attributes(_vault, _secret), do: exit(:nif_not_loaded)
    def set_secret_type(_vault, _secret, _secret_type), do: exit(:nif_not_loaded)
    def get_public_key(_vault, _secret), do: exit(:nif_not_loaded)
    def ecdh(_vault, _private_key, _peer_pubkey), do: exit(:nif_not_loaded)
    def hkdf_sha256(_vault, _salt, _ikm, _num_derived_outputs), do: exit(:nif_not_loaded)

    def aead_aes_gcm_encrypt(_vault, _key, _nonce, _additional_data, _plaintext),
      do: exit(:nif_not_loaded)

    def aead_aes_gcm_decrypt(_vault, _key, _nonce, _additional_data, _ciphertext_and_tag),
      do: exit(:nif_not_loaded)
  end
end
