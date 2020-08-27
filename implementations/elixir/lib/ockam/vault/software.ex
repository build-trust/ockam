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
end
