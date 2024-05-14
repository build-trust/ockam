defmodule OckamRustElixirNifs.Native do
  @moduledoc false

  version = Mix.Project.config()[:version]

  use RustlerPrecompiled,
    otp_app: :ockam_rust_elixir_nifs,
    crate: "ockam_rust_elixir_nifs",
    path: "../../../rust/ockam/ockam_rust_elixir_nifs",
    features: ["aws-lc"],
    force_build: System.get_env("OCKAM_DOWNLOAD_NIF") == nil,
    version: version,
    load_from: {:ockam_rust_elixir_nifs, "priv/native/libockam_rust_elixir_nifs"},
    # This is a fake link, I'll update after deploying a released nif
    base_url: "https://github.com/build-trust/ockam/releases/download/ockam_v#{version}"

  def create_identity, do: create_identity(nil)
  def create_identity(_), do: error()
  def check_identity(_), do: error()
  def attest_secure_channel_key(_, _), do: error()
  def verify_secure_channel_key_attestation(_, _, _), do: error()
  def verify_credential(_, _, _), do: error()
  def import_signing_secret(_), do: error()

  def setup_aws_kms(_), do: error()

  def issue_credential(_, _, _, _, _), do: error()

  defp error, do: :erlang.nif_error(:nif_not_loaded)
end
