defmodule Ockam.Vault.Software do
  @moduledoc """
  Ockam.Vault.Software
  """

  use Application

  defstruct [:id]

  @dialyzer :no_return

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

  def sha256(_vault, _input) do
    raise "natively implemented sha256/2 not loaded"
  end

  def secret_generate(_vault, _attributes) do
    raise "natively implemented secret_generate/2 not loaded"
  end

  def secret_import(_vault, _attributes, _input) do
    raise "natively implemented secret_import/3 not loaded"
  end

  def secret_export(_vault, _secret_handle) do
    raise "natively implemented secret_export/2 not loaded"
  end

  def secret_publickey_get(_vault, _secret_handle) do
    raise "natively implemented secret_publickey_get/2 not loaded"
  end

  def secret_attributes_get(_vault, _secret_handle) do
    raise "natively implemented secret_attributes_get/2 not loaded"
  end

  def secret_destroy(_vault, _secret_handle) do
    raise "natively implemented secret_destroy/2 not loaded"
  end

  def ecdh(_vault, _secret_handle, _input) do
    raise "natively implemented ecdh/3 not loaded"
  end

  def hkdf_sha256(_vault, _salt_handle, _ikm_handle, _derived_outputs_count) do
    raise "natively implemented hkdf_sha256/4 not loaded"
  end

  def hkdf_sha256(_vault, _salt_handle, _ikm_handle) do
    raise "natively implemented hkdf_sha256/3 not loaded"
  end

  def aead_aes_gcm_encrypt(_vault, _key_handle, _nonce, _ad, _plain_text) do
    raise "natively implemented aead_aes_gcm_encrypt/5 not loaded"
  end

  def aead_aes_gcm_decrypt(_vault, _key_handle, _nonce, _ad, _cipher_text) do
    raise "natively implemented aead_aes_gcm_decrypt/5 not loaded"
  end

  def deinit(_vault) do
    raise "natively implemented deinit/1 not loaded"
  end
end
