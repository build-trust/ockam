defmodule Ockam.Vault.Software do
  @moduledoc """
  Ockam.Vault.Software
  """

  use Application

  defstruct [:id]

  @on_load {:load_natively_implemented_functions, 0}

  app = Mix.Project.config()[:app]

  def load_natively_implemented_functions do
    path_components = [:code.priv_dir(unquote(app)), 'native', 'libockam_elixir_ffi']
    path = :filename.join(path_components)
    :ok = :erlang.load_nif(path, 0)
  end

  # Called when the Ockam application is started.
  #
  # This function is called when an application is started using
  # `Application.start/2`, `Application.ensure_started/2` etc.
  #
  @doc false
  def start(_type, _args) do
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

  def init do
    with {:ok, id} <- default_init() do
      {:ok, %__MODULE__{id: id}}
    end
  end

  def default_init do
    raise "natively implemented default_init/0 not loaded"
  end

  def file_init(_a) do
    raise "natively implemented file_init/1 not loaded"
  end

  def sha256(_a, _b) do
    raise "natively implemented sha256/2 not loaded"
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

  def ecdh(_a, _b, _c) do
    raise "natively implemented ecdh/3 not loaded"
  end

  def hkdf_sha256(_a, _b, _c, _d) do
    raise "natively implemented hkdf_sha256/4 not loaded"
  end

  def hkdf_sha256(_a, _b, _c) do
    raise "natively implemented hkdf_sha256/3 not loaded"
  end

  def aead_aes_gcm_encrypt(_a, _b, _c, _d, _e) do
    raise "natively implemented aead_aes_gcm_encrypt/5 not loaded"
  end

  def aead_aes_gcm_decrypt(_a, _b, _c, _d, _e) do
    raise "natively implemented aead_aes_gcm_decrypt/5 not loaded"
  end

  def get_persistence_id(_a, _b) do
    raise "natively implemented get_persistence_id/2 not loaded"
  end

  def get_persistent_secret(_a, _b) do
    raise "natively implemented get_persistent_secret/2 not loaded"
  end

  def deinit(_a) do
    raise "natively implemented deinit/1 not loaded"
  end

  def xx_initiator(_a, _b) do
    raise "natively implemented xx_initiator/2 not loaded"
  end

  def xx_responder(_a, _b) do
    raise "natively implemented xx_responder/2 not loaded"
  end

  def process(_a, _b) do
    raise "natively implemented process/2 not loaded"
  end

  def is_complete(_a) do
    raise "natively implemented is_complete/1 not loaded"
  end

  def finalize(_a) do
    raise "natively implemented finalize/1 not loaded"
  end
end
