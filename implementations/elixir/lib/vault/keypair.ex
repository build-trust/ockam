defmodule Ockam.Vault.KeyPair do
  @moduledoc "Public/private keypair abstraction"

  alias Ockam.Vault
  alias Ockam.Vault.Secret
  alias Ockam.Vault.SecretAttributes

  defstruct [:type, :priv, :pub]

  @type key_type :: :x25519
  @type t :: %__MODULE__{
          priv: Secret.t() | nil,
          pub: binary()
        }

  @doc """
  Generate a new keypair.

  If `public` is not provided, it will be computed from `secret`,
  if neither are provided, a new keypair will be generated via `:crypto`
  """
  @spec new(Vault.t(), SecretAttributes.t() | Keyword.t()) :: t()
  def new(vault, attrs_or_keys)

  def new(%Vault{} = vault, %SecretAttributes{} = attrs) do
    {:ok, secret} = Vault.generate_secret(vault, attrs)
    {:ok, pubkey} = Vault.get_public_key(vault, secret)
    %__MODULE__{type: attrs, priv: secret, pub: pubkey}
  end

  def new(%Vault{} = vault, keys) when is_list(keys) do
    priv =
      case Keyword.get(keys, :private) do
        nil ->
          nil

        bin when is_binary(bin) ->
          attrs = Keyword.fetch!(keys, :attrs)
          import_privkey(vault, attrs, bin)

        %Secret{} = priv ->
          priv
      end

    pub =
      case Keyword.get(keys, :public) do
        nil when not is_nil(priv) ->
          derive_pubkey(vault, priv)

        pub ->
          pub
      end

    %__MODULE__{priv: priv, pub: pub}
  end

  @doc false
  def from_hex(%Vault{} = vault, <<key::size(64)-binary>>) do
    <<priv::size(32)-binary>> = Base.decode16!(key, case: :mixed)
    new(vault, private: priv, attrs: SecretAttributes.x25519(:ephemeral))
  end

  @doc "Return the key type for this keypair"
  def key_type(%__MODULE__{priv: %Secret{attrs: attrs}}), do: SecretAttributes.type(attrs)
  def key_type(%__MODULE__{}), do: :buffer

  @doc "Return the public key for this keypair"
  def public_key(%__MODULE__{pub: pub}), do: pub

  @doc "Return the private key for this keypair"
  def private_key(%__MODULE__{priv: priv}), do: priv

  defp import_privkey(%Vault{} = vault, %SecretAttributes{} = attrs, privkey)
       when is_binary(privkey) do
    {:ok, secret} = Vault.import_secret(vault, privkey, attrs)
    secret
  end

  defp derive_pubkey(%Vault{} = vault, %Secret{} = priv) do
    {:ok, pub} = Vault.get_public_key(vault, priv)
    pub
  end
end
