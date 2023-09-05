defmodule Ockly.Native do

  use Rustler, otp_app: :ockly, crate: "ockly"

  def add(_, _), do: error()
  def create_identity(), do: error() 
  def check_identity(_), do: error()
  def attest_purpose_key(_, _), do: error()
  def verify_purpose_key_attestation(_, _, _), do: error()
  def verify_credential(_, _, _), do: error()

  def issue_credential(_, _, _, _), do: error()

  defp error, do: :erlang.nif_error(:nif_not_loaded)
end
