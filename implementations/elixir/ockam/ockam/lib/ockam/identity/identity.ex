defmodule Ockam.Identity do
  @moduledoc """
    build and work with Ockam Identities
  """

  alias Ockam.Credential.AttributeSet
  alias Ockam.Identity.Identifier
  alias __MODULE__

  @type identity_data() :: binary()

  defstruct [:identifier, :data]

  @type t() :: %Identity{}

  @type proof() :: binary()

  @type compare_result() :: :none | :equal | :conflict | :newer | :older

  @spec create() ::
          {:ok, identity :: t()} | {:error, reason :: any()}
  def create() do
    case Ockly.Native.create_identity() do
      {:error, reason} -> {:error, reason}
      {id, data} -> {:ok, %Identity{identifier: Identifier.from_str(id), data: data}}
    end
  end

  @spec create(secret_signing_key :: binary()) ::
          {:ok, identity :: t(), identitfier :: Identifier.t()} | {:error, reason :: any()}
  def create(secret) do
    key_id = Ockly.Native.import_signing_secret(secret)

    case Ockly.Native.create_identity(key_id) do
      {:error, reason} -> {:error, reason}
      {id, data} -> {:ok, %Identity{identifier: Identifier.from_str(id), data: data}}
    end
  end

  @spec import(contact_data :: binary(), secret_signing_key :: binary()) ::
          {:ok, identity :: t(), identifier :: Identifier.t()} | {:error, any()}
  def import(contact_data, secret_signing_key) do
    case Ockly.Native.import_signing_secret(secret_signing_key) do
      {:error, error} -> {:error, error}
      _key_id -> validate_contact_data(contact_data)
    end
  end

  @spec validate_contact_data(contact_data :: binary()) ::
          {:ok, identity :: t(), identifier :: Identifier.t()} | {:error, any()}
  def validate_contact_data(contact_data) do
    case Ockly.Native.check_identity(contact_data) do
      {:error, reason} ->
        {:error, reason}

      contact_id ->
        identifier = Identifier.from_str(contact_id)
        {:ok, %Identity{identifier: identifier, data: contact_data}, identifier}
    end
  end

  @spec get_data(t()) :: any()
  def get_data(%Identity{data: data}) do
    data
  end

  @spec get_identifier(t()) :: Identifier.t()
  def get_identifier(%Identity{identifier: id}) do
    id
  end

  # TODO: rename to attest_secure_channel_key
  @spec attest_purpose_key(contact :: t(), secret_key :: %{private: binary(), public: binary()}) ::
          {:ok, proof()} | {:error, any()}
  def attest_purpose_key(%Identity{identifier: identifier}, %{private: secret_key, public: _}) do
    case Ockly.Native.attest_secure_channel_key(Identifier.to_str(identifier), secret_key) do
      {:error, reason} -> {:error, reason}
      attestation -> {:ok, %Ockam.Identity.PurposeKeyAttestation{attestation: attestation}}
    end
  end

  # TODO: rename to verify_secure_channel_key_attestation
  @spec verify_purpose_key_attestation(
          contact :: t(),
          pubkey :: binary(),
          attestation :: Ockam.Identity.PurposeKeyAttestation.t()
        ) :: {:ok, boolean()} | {:error, any()}
  def verify_purpose_key_attestation(
        %Identity{data: identity_data},
        pubkey,
        %Ockam.Identity.PurposeKeyAttestation{attestation: attestation}
      ) do
    case Ockly.Native.verify_secure_channel_key_attestation(identity_data, pubkey, attestation) do
      {:error, reason} -> {:error, reason}
      true -> {:ok, true}
    end
  end

  # TODO refactor so that subject is an identity instead of identifier
  def issue_credential(%Identity{data: issuer}, %Identifier{} = subject, attrs, ttl)
      when is_map(attrs) do
    case Ockly.Native.issue_credential(issuer, Identifier.to_str(subject), attrs, ttl) do
      {:error, reason} -> {:error, reason}
      cred -> {:ok, cred}
    end
  end

  def verify_credential(%Identifier{} = subject_id, authorities, credential)
      when is_list(authorities) do
    authorities = Enum.map(authorities, fn a -> a.data end)

    case Ockly.Native.verify_credential(Identifier.to_str(subject_id), authorities, credential) do
      {:error, reason} ->
        {:error, reason}

      {expiration, verified_attrs} ->
        attributes = %AttributeSet{
          attributes: %AttributeSet.Attributes{attributes: verified_attrs},
          expiration: expiration
        }

        {:ok, attributes}
    end
  end

  @spec compare_identity_change_history(current_identity :: t(), known_identity :: t) ::
          {:ok, atom()} | {:error, reason :: any()}
  def compare_identity_change_history(_current_history, _known_history) do
    ## TODO:  implement change history compare!
    {:ok, :equal}
  end
end

defimpl CBOR.Encoder, for: Ockam.Identity do
  def encode_into(identity, acc) do
    <<acc::binary, Ockam.Identity.get_data(identity)::binary>>
  end
end
