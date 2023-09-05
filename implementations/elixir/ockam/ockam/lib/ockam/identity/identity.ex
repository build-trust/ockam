defmodule Ockam.Identity do
  @moduledoc """
  """

  @type identity_data() :: binary()
  @type identity_id() :: String.t()

  defstruct [:identity_id, :data]
  alias Ockam.Credential.AttributeSet
  alias Ockam.Identity

  @type t() :: %Identity{}

  @type proof() :: binary()

  @type compare_result() :: :none | :equal | :conflict | :newer | :older


  @spec create() ::
          {:ok, identity :: t(), identity_id :: binary()} | {:error, reason :: any()}
  def create() do
    with {id, data} <- Ockly.Native.create_identity() do
      {:ok, %Identity{identity_id: id, data: data}}
    end
  end

  @spec validate_contact_data(contact_data :: binary()) ::
          {:ok, identity :: t(), identity_id :: binary()}
  def validate_contact_data(contact_data) do
    with contact_id <- Ockly.Native.check_identity(contact_data) do
      {:ok, %Identity{identity_id: contact_id, data: contact_data},  contact_id}
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
          {:ok, proof()}
  def attest_purpose_key(%Identity{identity_id: identifier}, pubkey) do
    {:ok, %Ockam.Identity.PurposeKeyAttestation{attestation: Ockly.Native.attest_purpose_key(identifier, pubkey)}}
  end

  @spec verify_purpose_key_attestation(contact :: t(), pubkey :: binary(), attestation :: %Ockam.Identity.PurposeKeyAttestation{}) :: boolean()
  def verify_purpose_key_attestation(%Identity{data: identity_data}, pubkey, %Ockam.Identity.PurposeKeyAttestation{attestation: attestation}) do
    Ockly.Native.verify_purpose_key_attestation(identity_data,  pubkey, attestation)
  end


  def issue_credential(%Identity{data: issuer}, subject, attrs, ttl)  when is_map(attrs) and is_binary(subject) do
    cred = Ockly.Native.issue_credential(issuer, subject, attrs, ttl)
    IO.puts("> #{subject} #{inspect(cred)}")
    {:ok, cred}
  end

  def verify_credential(subject_id, authorities, credential) when is_binary(subject_id) and is_list(authorities) do
    IO.puts("< #{subject_id} #{inspect(credential)}")
    authorities = Enum.map(authorities, fn a -> a.data end)
    case  Ockly.Native.verify_credential(subject_id, authorities, credential) do
      {:error, reason} ->
        {:error, reason}
      {expiration, verified_attrs} ->
        IO.puts("< attributes:  #{inspect(verified_attrs)}")
        attributes = %Ockam.Credential.AttributeSet{attributes: %Ockam.Credential.AttributeSet.Attributes{attributes: verified_attrs},
                                  expiration: expiration}
        {:ok, attributes}
    end
  end
  """
  @spec compare_identity_change_history(current_identity :: t(), known_identity :: t) ::
          {:ok, atom()} | {:error, reason :: any()}
  def compare_identity_change_history({module, current_data}, {module, known_data}) do
    module.compare_identity_change_history(current_data, known_data)
  end

  def compare_identity_change_history(current_identity, known_identity) do
    {:error, {:different_identity_implementations, current_identity, known_identity}}
  end
  """
end

  defimpl CBOR.Encoder, for: Ockam.Identity do
    def encode_into(identity, acc) do
     <<acc::binary,  (Ockam.Identity.get_data(identity))::binary>>
    end
  end
