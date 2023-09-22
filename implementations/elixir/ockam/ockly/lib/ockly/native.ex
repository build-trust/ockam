defmodule Ockly.Native do
  @moduledoc false

  use Rustler,
    otp_app: :ockly,
    crate: "ockly",
    load_data_fun: {Ockly, :nif_config},
    skip_compilation?: System.get_env("OCKLY_PRECOMPILED_LIB", "true") == "true",
    load_from: {:ockly, "priv/native/libockly"}

  def create_identity, do: create_identity(nil)

  @spec create_identity(binary() | nil) :: {binary(), binary()}
  def create_identity(_), do: error()

  @spec check_identity(binary()) :: binary() | {:error, term()}
  def check_identity(_), do: error()

  @spec attest_secure_channel_key(binary(), binary()) :: binary() | {:error, term()}
  def attest_secure_channel_key(_, _), do: error()

  def verify_secure_channel_key_attestation(_, _, _), do: error()
  def verify_credential(_, _, _), do: error()
  def import_signing_secret(_), do: error()

  def issue_credential(_, _, _, _), do: error()

  defp error, do: :erlang.nif_error(:nif_not_loaded)
end
