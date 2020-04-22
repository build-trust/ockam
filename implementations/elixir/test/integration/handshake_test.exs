defmodule Ockam.Integration.Handshake.Test do
  use ExUnit.Case, async: false
  require Logger

  alias Ockam.Channel
  alias Ockam.Transport.Address
  alias Ockam.Transport.Socket
  alias Ockam.Vault.KeyPair
  alias Ockam.Router.Protocol.Message
  alias Ockam.Router.Protocol.Message.Envelope
  alias Ockam.Router.Protocol.Encoding

  setup context do
    if transport = context[:transport] do
      name = Map.fetch!(context, :transport_name)
      meta = [name: name]
      config = Map.get(context, :transport_config, [])
      pid = start_supervised!({transport, [meta, config]})
      {:ok, [pid: pid, config: config]}
    else
      {:ok, []}
    end
  end

  @tag skip: true
  @tag transport: Ockam.Transport.TCP
  @tag transport_name: :tcp_4002
  @tag transport_config: [listen_address: "0.0.0.0", listen_port: 4002]
  @tag capture_log: false
  test "with C implementation as initiator", %{config: config} do
    port = config[:listen_port]
    assert {:ok, _} = run_initiator!(["-a", "127.0.0.1", "-p", "#{port}"])
    assert :ok = await_test_executable()
  end

  @tag skip: true
  @tag initiator: true
  @tag listen_port: 4003
  @tag capture_log: false
  test "with C implementation as responder", %{listen_port: port} do
    # Start server first
    assert {:ok, _} = run_responder!(["-a", "127.0.0.1", "-p", "#{port}"])

    {:ok, addr} = Address.new(:inet, :loopback, port)
    socket = Socket.new(:client, addr)

    s = KeyPair.new(:x25519)
    e = KeyPair.new(:x25519)
    rs = KeyPair.new(:x25519)
    re = KeyPair.new(:x25519)

    handshake_opts = %{protocol: "Noise_XX_25519_AESGCM_SHA256", s: s, e: e, rs: rs, re: re}
    assert {:ok, handshake} = Channel.handshake(:initiator, handshake_opts)
    assert {:ok, transport} = Socket.open(socket)

    assert {:ok, chan, transport} =
             Channel.negotiate_secure_channel(handshake, transport, %{timeout: 10_000})

    assert {:ok, message, encrypted} = Socket.recv(transport, timeout: 10_000)
    assert {:ok, chan, encoded} = Channel.decrypt(chan, encrypted)
    assert {:ok, %Envelope{body: %Message.Ping{}}} = Encoding.decode(encoded)

    assert {:ok, encoded} = Encoding.encode(%Message.Pong{})
    assert {:ok, chan, encrypted} = Channel.encrypt(chan, encoded)
    assert {:ok, transport} = Socket.send(transport, encrypted)
    assert {:ok, _} = Socket.close(transport)
    assert :ok = await_test_executable()
  end

  defp run_initiator!(args) when is_list(args) do
    invoke_test_executable!(["-i" | args])
  end

  defp run_responder!(args) when is_list(args) do
    invoke_test_executable!(["-r" | args])
  end

  defp invoke_test_executable!(args) when is_list(args) do
    parent = self()

    spawn_link(fn ->
      Process.flag(:trap_exit, true)

      init_dir = System.get_env("OCKAM_C_BUILD_DIR")
      init_dir = init_dir || Path.join([__DIR__, "..", "..", "..", "c", "_build"])
      init_dir = Path.expand(init_dir)
      init_cmd = Path.join([init_dir, "Debug", "tests", "ockam_key_agreement_tests_xx_full"])
      Logger.debug("[port] spawning #{init_cmd} with args #{inspect(args)}")

      port =
        Port.open({:spawn_executable, init_cmd}, [
          :binary,
          :exit_status,
          :stderr_to_stdout,
          line: 200,
          args: args
        ])

      Logger.debug("[port] spawned #{inspect(port)}")

      # Give program 50ms to start
      Process.sleep(50)

      send(parent, :spawned)
      monitor_test_executable(parent, port, <<>>)
    end)

    receive do
      :spawned ->
        {:ok, ""}

      {:exit_status, 0} ->
        {:ok, ""}

      {:exit_status, status, output} ->
        Logger.warn("Captured Output:\n" <> output)
        {:error, status}

      {:ok, _output} = result ->
        result

      {:error, reason, output} ->
        Logger.warn("Captured Output:\n" <> output)
        {:error, reason}
    end
  end

  defp monitor_test_executable(parent, port, output) do
    receive do
      {^port, {:data, {_flag, data}}} ->
        Logger.debug("[responder #{inspect(port)}] #{data}")
        monitor_test_executable(parent, port, output <> "\n" <> data)

      {^port, {:exit_status, status}} ->
        send(port, {self(), :close})
        send(parent, {:exit_status, status, output})
        monitor_test_executable(parent, port, output)

      {^port, :closed} ->
        Logger.debug("[port] #{inspect(port)} closed")

        unless is_nil(parent) do
          send(parent, {:ok, output})
        end

        exit(:normal)

      {:EXIT, ^port, reason} ->
        Logger.debug("[port] #{inspect(port)} exited #{inspect(reason)}")

        unless is_nil(parent) do
          send(parent, {:error, reason, output})
        end

        exit(:normal)

      {:EXIT, ^parent, _} ->
        send(port, {self(), :close})
        monitor_test_executable(nil, port, output)
    end
  end

  defp await_test_executable() do
    receive do
      {:exit_status, 0, _} ->
        :ok

      {:exit_status, status, output} ->
        Logger.warn("Captured Output:\n" <> output)
        {:error, status}

      {:error, reason, output} ->
        Logger.warn("Captured Output:\n" <> output)
        {:error, reason}
    end
  end
end
