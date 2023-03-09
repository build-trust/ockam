defmodule Ockam.Healthcheck.Application.Test do
  use ExUnit.Case, async: true

  require Logger

  test "parse target config" do
    full_target = """
    [{"name": "mytarget",
      "host":"localhost",
      "port": 4000,
      "api_worker": "api",
      "healthcheck_worker": "healthcheck"}]
    """

    {:ok, _targets} = Ockam.Healthcheck.Application.parse_config(full_target)

    simple_target = """
    [{"name": "mytarget",
      "host": "localhost",
      "port": 4000}]
    """

    {:ok, [target]} = Ockam.Healthcheck.Application.parse_config(simple_target)

    ## Set fields
    assert %{name: "mytarget", host: "localhost", port: 4000} = target

    ## Default fields
    assert %{api_worker: "api", healthcheck_worker: "healthcheck"} = target

    bad_target = "[{\"host\":\"localhost\"}]"

    assert {:error, {:invalid_target, _error}} =
             Ockam.Healthcheck.Application.parse_config(bad_target)

    bad_config = "not json"

    assert {:error, {:invalid_config, _error}} =
             Ockam.Healthcheck.Application.parse_config(bad_config)
  end
end
