defmodule Ockam.Vault.Software do
  @moduledoc """
  Ockam.Vault.Software
  """

  use Application

  @on_load :load_natively_implemented_functions

  require Logger

  # Called when the Ockam application is started.
  #
  # This function is called when an application is started using
  # `Application.start/2`, `Application.ensure_started/2` etc.
  #
  @doc false
  def start(_type, _args) do
    Logger.info("Starting #{__MODULE__}")

    # Specifications of child processes that will be started and supervised.
    #
    # See the "Child specification" section in the `Supervisor` module for more
    # detailed information.
    children = []

    # Start a supervisor with the given children. The supervisor will inturn
    # start the given children.
    #
    # The :one_for_one supervision strategy is used, if a child process
    # terminates, only that process is restarted.
    #
    # See the "Strategies" section in the `Supervisor` module for more
    # detailed information.
    Supervisor.start_link(children, strategy: :one_for_one, name: __MODULE__)
  end

  def load_natively_implemented_functions do
    [:code.priv_dir(:ockam_vault_software), "native", "libockam_elixir_vault_software"]
    |> Path.join()
    |> to_charlist()
    |> :erlang.load_nif(0)
  end

  def default_init do
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

  def secret_publickey_get(_a, _b) do
    raise "natively implemented secret_publickey_get/2 not loaded"
  end

  def secret_attributes_get(_a, _b) do
    raise "natively implemented secret_attributes_get/2 not loaded"
  end

  def secret_destroy(_a, _b) do
    raise "natively implemented secret_destroy/2 not loaded"
  end
end