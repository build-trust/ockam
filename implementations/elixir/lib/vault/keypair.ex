defmodule Ockam.Vault.KeyPair do
  @moduledoc "Public/private keypair abstraction"

  @key_type :x25519
  @key_types [@key_type]

  defstruct [:type, :priv, :pub]

  @type key_type :: :x25519
  @type t :: %__MODULE__{
          type: key_type,
          priv: binary() | nil,
          pub: binary()
        }

  @doc """
  Generate a new keypair.

  If `public` is not provided, it will be computed from `secret`,
  if neither are provided, a new keypair will be generated via `:crypto`
  """
  @spec new(key_type()) :: t()
  @spec new(key_type(), Keyword.t()) :: t()
  def new(type) when type in @key_types do
    {pub, priv} = :crypto.generate_key(:ecdh, type)
    %__MODULE__{type: @key_type, priv: priv, pub: pub}
  end

  def new(type, keys) when type in @key_types when is_list(keys) do
    priv = Keyword.get(keys, :private)

    pub =
      case Keyword.get(keys, :public) do
        nil when not is_nil(priv) ->
          derive_pubkey(type, priv)

        pub ->
          pub
      end

    %__MODULE__{type: type, priv: priv, pub: pub}
  end

  @doc false
  def from_hex(<<key::size(64)-binary>>) do
    <<priv::size(32)-binary>> = Base.decode16!(key, case: :mixed)
    pub = derive_pubkey(:x25519, priv)
    %__MODULE__{type: :x25519, priv: priv, pub: pub}
  end

  @doc "Return the key type for this keypair"
  def key_type(%__MODULE__{type: type}), do: type

  @doc "Return the public key for this keypair"
  def public_key(%__MODULE__{pub: pub}), do: pub

  @doc "Return the private key for this keypair"
  def private_key(%__MODULE__{priv: priv}), do: priv

  def derive_pubkey(@key_type, priv) do
    :enacl.curve25519_scalarmult_base(priv)
  end
end
