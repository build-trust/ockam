defmodule Ockam.Session.Tests do
  use ExUnit.Case, async: true

  alias Ockam.Session.Pluggable, as: Session

  alias Ockam.Session.Tests.DataModule
  alias Ockam.Session.Tests.Handshake

  test "Initiator sends init message on start" do
    {:ok, me} = Ockam.Node.register_random_address()

    {:ok, initiator} =
      Session.Initiator.create(
        init_route: [me],
        handshake_mod: Handshake,
        data_worker_mod: DataModule
      )

    on_exit(fn ->
      Ockam.Node.stop(initiator)
    end)

    receive do
      %{payload: "init"} ->
        :ok
    after
      5000 ->
        raise "Did not receive init message"
    end
  end

  test "Responder handles init message" do
    {:ok, me} = Ockam.Node.register_random_address()

    {:ok, responder} =
      Session.Responder.create(
        handshake_mod: Handshake,
        data_worker_mod: DataModule,
        handshake_options: [reply_to: [me]]
      )

    {:ok, responder_inner} = Ockam.AsymmetricWorker.get_inner_address(responder)

    {:ok, initiator} =
      Session.Initiator.create(
        init_route: [responder_inner],
        handshake_mod: Handshake,
        data_worker_mod: DataModule
      )

    on_exit(fn ->
      Ockam.Node.stop(initiator)
      Ockam.Node.stop(responder)
    end)

    receive do
      %{payload: "responder reply"} ->
        :ok
    after
      5000 ->
        raise "Did not receive responder message"
    end
  end

  test "Initiator handles handshake message" do
    {:ok, me} = Ockam.Node.register_random_address()

    {:ok, responder} =
      Session.Responder.create(
        handshake_mod: Handshake,
        data_worker_mod: DataModule
      )

    {:ok, responder_inner} = Ockam.AsymmetricWorker.get_inner_address(responder)

    {:ok, initiator} =
      Session.Initiator.create(
        init_route: [responder_inner],
        data_worker_mod: DataModule,
        handshake_mod: Handshake,
        handshake_options: [reply_to: [me]]
      )

    on_exit(fn ->
      Ockam.Node.stop(initiator)
      Ockam.Node.stop(responder)
    end)

    receive do
      %{payload: "initiator reply"} ->
        :ok
    after
      5000 ->
        raise "Did not receive initiator message"
    end
  end

  test "Routing example run" do
    state = Ockam.Examples.Session.Routing.run()

    receive do
      %{payload: "Hi me!"} ->
        :ok
    after
      1000 ->
        raise "Did not receive forwarded message from Routing example"
    end

    assert Map.fetch!(state, :stage) == :data
  end

  test "CountTo example run" do
    state = Ockam.Examples.Session.CountTo.run()

    receive do
      %{payload: "10"} ->
        :ok
    after
      1000 ->
        raise "Did not receive forwarded message from CountTo example"
    end

    assert Map.fetch!(state, :stage) == :data
    assert state |> Map.fetch!(:data_state) |> Map.fetch!(:count) == 10
  end

  test "PipeChannel example run" do
    state = Ockam.Examples.Session.PipeChannel.run()

    receive do
      %{payload: "Hi me!"} ->
        :ok
    after
      1000 ->
        raise "Did not receive forwarded message from PipeChannel example"
    end

    assert Map.fetch!(state, :stage) == :data
  end
end
