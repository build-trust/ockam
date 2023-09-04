defmodule Ockam.Healthcheck.Test do
  use ExUnit.Case

  alias Ockam.Healthcheck.APIEndpointTarget
  alias Ockam.Healthcheck.Target

  require Logger

  setup_all do
    start_supervised({Ockam.Transport.TCP, [listen: [port: 4000]]})
    {:ok, identity} = Ockam.Identity.create()
    {:ok, keypair} = Ockam.SecureChannel.Crypto.generate_dh_keypair()
    {:ok, attestation} = Ockam.Identity.attest_purpose_key(identity, keypair)

    {:ok, _api} =
      Ockam.SecureChannel.create_listener(
        address: "api",
        identity: identity,
        encryption_options: [static_keypair: keypair, static_key_attestation: attestation],
        trust_policies: []
      )

    {:ok, _pid, _ping} = Ockam.Services.Echo.start_link(address: "healthcheck")
    {:ok, _api} = Ockam.Healthcheck.TestAPIEndpoint.create(address: "endpoint")

    on_exit(fn ->
      Ockam.Node.stop("api")
      Ockam.Node.stop("healthcheck")
      Ockam.Node.stop("endpoint")
    end)

    :ok
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

  test "healthcheck API endpoint target OK" do
    target = %APIEndpointTarget{
      name: "target",
      host: "localhost",
      port: 4000,
      api_worker: "api",
      healthcheck_worker: "endpoint",
      path: "/ok",
      method: :get
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
                   1000

    assert_receive {:telemetry_event, [:ockam, :healthcheck, :ok], %{duration: _duration},
                    %{target: %{name: "target"}}},
                   500

    refute_received {:telemetry_event, [:ockam, :healthcheck, :error], %{duration: _duration},
                     %{target: %{name: "target"}}}
  end

  test "healthcheck API endpoint target Error" do
    target = %APIEndpointTarget{
      name: "target",
      host: "localhost",
      port: 4000,
      api_worker: "api",
      healthcheck_worker: "endpoint",
      path: "/error",
      method: :get
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

    assert {:error, _reason} = Ockam.Healthcheck.check_target(target, 1000)

    assert_receive {:telemetry_event, [:ockam, :healthcheck, :result], %{status: 0},
                    %{target: %{name: "target"}}},
                   1000

    assert_receive {:telemetry_event, [:ockam, :healthcheck, :error], %{duration: _duration},
                    %{target: %{name: "target"}}},
                   500

    refute_received {:telemetry_event, [:ockam, :healthcheck, :ok], %{duration: _duration},
                     %{target: %{name: "target"}}}
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
