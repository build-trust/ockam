defmodule Ockam.Vault.SecretAttributes do
  defstruct [:length, :ty, :purpose, :persistence]

  @type secret_persistence :: :static | :ephemeral
  @type secret_type :: :unspecified | :aes128 | :aes256 | :curve25519_private | :p256_private
  @type secret_purpose :: :key_agreement

  @type t :: %__MODULE__{
          length: non_neg_integer(),
          ty: secret_type(),
          purpose: secret_purpose(),
          persistence: secret_persistence()
        }

  @persistence_types [:ephemeral, :static]
  @secret_types [:unspecified, :aes128, :aes256, :curve25519_private, :p256_private]

  @doc "Get the type of secret this represents"
  @spec type(t) :: secret_type()
  def type(%__MODULE__{ty: ty}), do: ty

  @doc "Get the default attributes for a Curve25519 private key"
  @spec x25519(secret_persistence()) :: t
  def x25519(persistence) when persistence in @persistence_types do
    %__MODULE__{
      length: 0,
      ty: :curve25519_private,
      purpose: :key_agreement,
      persistence: persistence
    }
  end

  @doc "Get a default set of attributes for a secret of unspecified nature"
  @spec x25519(secret_persistence()) :: t
  def unspecified(persistence) when persistence in @persistence_types do
    %__MODULE__{
      length: 0,
      ty: :unspecified,
      purpose: :key_agreement,
      persistence: persistence
    }
  end

  @spec set_type(t, secret_type()) :: {:ok, t} | {:error, term}
  def set_type(%__MODULE__{} = attrs, type) when type in @secret_types do
    {:ok, %__MODULE__{attrs | ty: type}}
  end

  def set_type(%__MODULE__{}, _type), do: {:error, :invalid_secret_type}

  @doc "Get the default attributes for a secret of the given type"
  @spec from_type(secret_type(), secret_persistence()) :: t
  def from_type(type, persistence \\ :ephemeral)

  def from_type(:x25519, :ephemeral), do: x25519(:ephemeral)
  def from_type(:unspecified, :ephemeral), do: unspecified(:ephemeral)
end
