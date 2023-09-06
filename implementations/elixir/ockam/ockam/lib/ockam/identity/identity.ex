defmodule Ockam.Identity do
  @moduledoc """
  """

  @type identity_data() :: binary()
  @type identity_id() :: String.t()

  defstruct [:identity_id, :data]
  alias Ockam.Identity
  alias Ockam.Credential.AttributeSet

  @type t() :: %Identity{}

  @type proof() :: binary()

  @type compare_result() :: :none | :equal | :conflict | :newer | :older


  @spec create() ::
          {:ok, identity :: t(), identity_id :: binary()} | {:error, reason :: any()}
  def create() do
    case Ockly.Native.create_identity() do
      {:error, reason} -> {:error, reason}
      {id, data} -> {:ok, %Identity{identity_id: id, data: data}}
    end
  end

  @spec import(contact_data :: binary(), secret_signing_key :: binary()) ::
          {:ok, identity :: t(), identity_id :: binary()} | {:error, any()}
  def import(contact_data, secret_signing_key) do
    case Ockly.Native.import_signing_secret(secret_signing_key) do
      {:error, error} -> {:error, error}
      _key_id -> validate_contact_data(contact_data)
    end
  end

  @spec validate_contact_data(contact_data :: binary()) ::
          {:ok, identity :: t(), identity_id :: binary()} | {:error, any()}
  def validate_contact_data(contact_data) do
    case Ockly.Native.check_identity(contact_data) do
      {:error, reason} -> {:error, reason}
      contact_id  -> {:ok, %Identity{identity_id: contact_id, data: contact_data},  contact_id}
    end
  end

  @spec get_data(t()) :: any()
  def get_data(%Identity{data: data}) do
    data
  end

  @spec get_identifier(t()) :: String.t()
  def get_identifier(%Identity{identity_id: id}) do
    id
  end

  @spec attest_purpose_key(contact :: t(), pubkey :: binary()) ::
          {:ok, proof()} | {:error, any()}
  def attest_purpose_key(%Identity{identity_id: identifier}, pubkey) do
    case Ockly.Native.attest_purpose_key(identifier, pubkey) do
      {:error, reason} -> {:error, reason}
      attestation -> {:ok, %Ockam.Identity.PurposeKeyAttestation{attestation: attestation}}
    end
  end

  @spec verify_purpose_key_attestation(contact :: t(), pubkey :: binary(), attestation :: %Ockam.Identity.PurposeKeyAttestation{}) :: {:ok, boolean()} | {:error, any()}
  def verify_purpose_key_attestation(%Identity{data: identity_data}, pubkey, %Ockam.Identity.PurposeKeyAttestation{attestation: attestation}) do
    case Ockly.Native.verify_purpose_key_attestation(identity_data,  pubkey, attestation) do
      {:error, reason} -> {:error, reason}
      true -> {:ok, true}
    end
  end


  def issue_credential(%Identity{data: issuer}, subject, attrs, ttl)  when is_map(attrs) and is_binary(subject) do
    case Ockly.Native.issue_credential(issuer, subject, attrs, ttl) do
      {:error, reason} -> {:error, reason}
      cred -> {:ok, cred}
    end
  end

  def verify_credential(subject_id, authorities, credential) when is_binary(subject_id) and is_list(authorities) do
    authorities = Enum.map(authorities, fn a -> a.data end)
    case  Ockly.Native.verify_credential(subject_id, authorities, credential) do
      {:error, reason} ->
        {:error, reason}
      {expiration, verified_attrs} ->
        attributes = %AttributeSet{attributes: %AttributeSet.Attributes{attributes: verified_attrs},
                                  expiration: expiration}
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
     <<acc::binary,  (Ockam.Identity.get_data(identity))::binary>>
    end
  end
