defmodule Ockam.Identity.API.Tests do
  use ExUnit.Case

  alias Ockam.Identity.API.Request
  alias Ockam.Identity.API.Response

  require Logger

  describe "Request" do
    test "validate_identity_change_history" do
      identity = "my identity"
      data = Request.validate_identity_change_history(identity)
      Logger.info("#{inspect(data)}")
      {:ok, %{identity: ^identity}} = Request.decode_validate_identity_change_history(data)
    end

    test "create_signature" do
      identity = "my identity"
      hash = "my hash"
      data = Request.create_signature(nil, identity, hash)
      Logger.info("#{inspect(data)}")
      {:ok, %{identity: ^identity, state: ^hash}} = Request.decode_create_signature(data)
    end

    test "verify_signature" do
      identity = "my identity"
      hash = "my hash"
      proof = "my proof"
      data = Request.verify_signature(identity, proof, hash)
      Logger.info("#{inspect(data)}")

      {:ok, %{identity: ^identity, state: ^hash, proof: ^proof}} =
        Request.decode_verify_signature(data)
    end

    test "compare_identity_change_history" do
      identity = "my identity"
      known_identity = "other identity"
      data = Request.compare_identity_change_history(identity, known_identity)
      Logger.info("#{inspect(data)}")

      {:ok, %{identity: ^identity, known_identity: ^known_identity}} =
        Request.decode_compare_identity_change_history(data)
    end
  end

  describe "Response" do
    test "create" do
      identity = "my identity"
      identity_id = "my_id"
      data = Response.encode_create(%{identity: identity, identity_id: identity_id})
      {:ok, %{identity: ^identity, identity_id: ^identity_id}} = Response.create(data)
    end

    test "validate_identity_change_history" do
      identity_id = "my_id"
      data = Response.encode_validate_identity_change_history(%{identity_id: identity_id})
      {:ok, %{identity_id: ^identity_id}} = Response.validate_identity_change_history(data)
    end

    test "create_signature" do
      proof = "my proof"
      data = Response.encode_create_signature(%{proof: proof})
      {:ok, %{proof: ^proof}} = Response.create_signature(data)
    end

    test "verify_signature" do
      data = Response.encode_verify_signature(%{verified: true})
      {:ok, %{verified: true}} = Response.verify_signature(data)
    end

    test "compare_identity_change_history" do
      data = Response.encode_compare_identity_change_history(:conflict)
      {:ok, :conflict} = Response.compare_identity_change_history(data)
    end
  end
end
