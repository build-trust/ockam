defmodule Start do
  @doc """
  Start a local project node with the ability to create secure channels.
  The node identity is either retrieved from files, or created then stored in files.
  """
  def start_node() do
    with {:ok, own_identity} <- get_or_create_identity(),
         {:ok, keypair} <- Ockam.SecureChannel.Crypto.generate_dh_keypair(),
         {:ok, attestation} <- Ockam.Identity.attest_purpose_key(own_identity, keypair) do
      Ockam.Services.start_service(
        :secure_channel,
        identity: own_identity,
        address: "api",
        encryption_options: [static_keypair: keypair, static_key_attestation: attestation]
      )
    end
  end

  @doc """
  Retrieve or create an identity
  """
  def get_or_create_identity() do
    case File.read(identity_path()) do
      {:ok, bytes} ->
        with {:ok, secret_bytes} <- File.read(secret_signing_key_path()),
             {:ok, identity, _identifier} <- Ockam.Identity.import(bytes, secret_bytes) do
          {:ok, identity}
        end

      {:error, :enoent} ->
        with {_pub, secret} <- :crypto.generate_key(:eddsa, :ed25519),
             {:ok, identity} <- Ockam.Identity.create(secret),
             :ok <- File.mkdir_p(Path.dirname(identity_id_path())),
             :ok <-
               File.write(
                 identity_id_path(),
                 Ockam.Identity.Identifier.to_str(Ockam.Identity.get_identifier(identity))
               ),
             :ok <- File.write(identity_path(), Ockam.Identity.get_data(identity)),
             :ok <- File.write(secret_signing_key_path(), secret) do
          {:ok, identity}
        end
    end
  end

  @doc """
  File storing the identity identifier
  """
  defp identity_id_path() do
    Path.join(Application.fetch_env!(:ockam_cloud_node, :storage_path), "identity.id")
  end

  @doc """
  File storing the identity change history
  """
  defp identity_path() do
    Path.join(Application.fetch_env!(:ockam_cloud_node, :storage_path), "identity")
  end

  @doc """
  File storing the identity secret
  """
  defp secret_signing_key_path() do
    Path.join(Application.fetch_env!(:ockam_cloud_node, :storage_path), "identity.secret")
  end
end

Start.start_node()
