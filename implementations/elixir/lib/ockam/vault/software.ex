defmodule Ockam.Vault.Software do
  @on_load :load_natively_implemented_functions

  def load_natively_implemented_functions do
    [:code.priv_dir(:ockam), "native", "libockam_elixir_vault_software"]
    |> Path.join()
    |> to_charlist()
    |> :erlang.load_nif(0)
  end

  def default_init() do
    raise "natively implemented default_init/0 not loaded"
  end

  def sha256(_a, _b) do
    raise "natively implemented sha256/2 not loaded"
  end

  def random_bytes(_a, _b) do
    raise "natively implemented random_bytes/2 not loaded"
  end

  def secret_generate(_a, _b) do
    raise "natively implemented secret_generate/2 not loaded"
  end

  def secret_import(_a, _b, _c) do
    raise "natively implemented secret_import/3 not loaded"
  end

  def secret_export(_a, _b) do
    raise "natively implemented secret_export/2 not loaded"
  end
end
