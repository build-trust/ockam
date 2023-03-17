defmodule Ockam.Healthcheck.Test do
  use ExUnit.Case

  alias Ockam.Healthcheck.Target

  require Logger

  setup_all do
    {:ok, transport} = Ockam.Transport.TCP.start(listen: [port: 4000])

    {:ok, _api} =
      Ockam.Identity.SecureChannel.create_listener(
        identity: :dynamic,
        address: "api",
        trust_policies: []
      )

    {:ok, _pid, _ping} = Ockam.Services.Echo.start_link(address: "healthcheck")

    on_exit(fn ->
      Ockam.Node.stop("api")
      Ockam.Node.stop("healthcheck")
      GenServer.stop(transport)
    end)

    :ok
  end

  test "healthcheck targets OK" do
    target = %Target{
      name: "target",
      host: "localhost",
      port: 4000,
      api_worker: "api",
      healthcheck_worker: "healthcheck"
    }

    old = Application.get_env(:ockam_healthcheck, :targets, [])
    Application.put_env(:ockam_healthcheck, :targets, [target, target])

    on_exit(fn ->
      Application.put_env(:ockam_healthcheck, :targets, old)
    end)

    assert :ok = Ockam.Healthcheck.check_targets()
  end

  test "healthcheck target OK" do
    target = %Target{
      name: "target",
      host: "localhost",
      port: 4000,
      api_worker: "api",
      healthcheck_worker: "healthcheck"
    }

    test_proc = self()

    :telemetry.attach_many(
      "test_handler",
      [
        [:ockam, :healthcheck, :result],
        [:ockam, :healthcheck, :ok],
        [:ockam, :healthcheck, :error]
      ],
      fn event, measurements, metadata, _config ->
        send(test_proc, {:telemetry_event, event, measurements, metadata})
      end,
      nil
    )

    on_exit(fn ->
      :telemetry.detach("test_handler")
    end)

    assert :ok = Ockam.Healthcheck.check_target(target, 1000)

    assert_receive {:telemetry_event, [:ockam, :healthcheck, :result], %{status: 1},
                    %{target: %{name: "target"}}},
                   5000

    assert_receive {:telemetry_event, [:ockam, :healthcheck, :ok], %{duration: _duration},
                    %{target: %{name: "target"}}},
                   5000

    refute_receive {:telemetry_event, [:ockam, :healthcheck, :error], %{duration: _duration},
                    %{target: %{name: "target"}}},
                   500
  end

  test "healthcheck ping error" do
    target = %Target{
      name: "target",
      host: "localhost",
      port: 4000,
      api_worker: "api",
      healthcheck_worker: "not_healthcheck"
    }

    test_proc = self()

    :telemetry.attach_many(
      "test_handler",
      [
        [:ockam, :healthcheck, :result],
        [:ockam, :healthcheck, :ok],
        [:ockam, :healthcheck, :error]
      ],
      fn event, measurements, metadata, _config ->
        send(test_proc, {:telemetry_event, event, measurements, metadata})
      end,
      nil
    )

    on_exit(fn ->
      :telemetry.detach("test_handler")
    end)

    assert {:error, :timeout} = Ockam.Healthcheck.check_target(target, 1000)

    assert_receive {:telemetry_event, [:ockam, :healthcheck, :result], %{status: 0},
                    %{target: %{name: "target"}}},
                   1000

    assert_receive {:telemetry_event, [:ockam, :healthcheck, :error], %{duration: _duration},
                    %{target: %{name: "target"}}},
                   1000

    refute_receive {:telemetry_event, [:ockam, :healthcheck, :ok], %{duration: _duration},
                    %{target: %{name: "target"}}},
                   500
  end

  test "healthcheck channel error" do
    target = %Target{
      name: "target",
      host: "localhost",
      port: 4000,
      api_worker: "not_api",
      healthcheck_worker: "healthcheck"
    }

    assert {:error, {:secure_channel_error, :key_exchange_timeout}} =
             Ockam.Healthcheck.check_target(target, 1000)
  end

  test "healthcheck TCP error" do
    target = %Target{
      name: "target",
      host: "localhost",
      port: 1234,
      api_worker: "api",
      healthcheck_worker: "healthcheck"
    }

    assert {:error, {:tcp_connection_error, :econnrefused}} =
             Ockam.Healthcheck.check_target(target, 1000)
  end
end
