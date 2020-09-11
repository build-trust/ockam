defmodule Ockam.Kex.Rust do
  @moduledoc """
  Ockam.Kex.Rust
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
    [:code.priv_dir(:ockam_vault_software), "native", "libockam_elixir_kex_rust"]
    |> Path.join()
    |> to_charlist()
    |> :erlang.load_nif(0)
  end

  def kex_init_initiator(_a) do
    raise "natively implemented kex_init_initiator/1 not loaded"
  end

  def kex_init_responder(_a) do
    raise "natively implemented kex_init_responder/1 not loaded"
  end

  def kex_initiator_encode_message_1(_a, _b) do
    raise "natively implemented kex_initiator_encode_message_1/2 not loaded"
  end

  def kex_responder_encode_message_2(_a, _b) do
    raise "natively implemented kex_responder_encode_message_2/2 not loaded"
  end

  def kex_initiator_encode_message_3(_a, _b) do
    raise "natively implemented kex_initiator_encode_message_3/2 not loaded"
  end

  def kex_responder_decode_message_1(_a, _b) do
    raise "natively implemented kex_responder_decode_message_1/2 not loaded"
  end

  def kex_initiator_decode_message_2(_a, _b) do
    raise "natively implemented kex_initiator_decode_message_2/2 not loaded"
  end

  def kex_responder_decode_message_3(_a, _b) do
    raise "natively implemented kex_responder_decode_message_3/2 not loaded"
  end

  def kex_initiator_finalize(_a) do
    raise "natively implemented kex_initiator_finalize/1 not loaded"
  end

  def kex_responder_finalize(_a) do
    raise "natively implemented kex_responder_finalize/1 not loaded"
  end
end