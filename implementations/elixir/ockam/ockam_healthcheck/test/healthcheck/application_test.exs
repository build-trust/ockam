defmodule Ockam.Healthcheck.Application.Test do
  use ExUnit.Case, async: true

  alias Ockam.Healthcheck.APIEndpointTarget
  alias Ockam.Healthcheck.Target

  require Logger

  test "parse target config" do
    full_target = """
    [{"name": "mytarget",
      "host":"localhost",
      "port": 4000,
      "api_worker": "api",
      "healthcheck_worker": "healthcheck",
      "crontab": "* * * * *"}]
    """

    {:ok, _targets} = Ockam.Healthcheck.Application.parse_config(full_target)

    simple_target = """
    [{"name": "mytarget",
      "host": "localhost",
      "port": 4000,
      "crontab": "* * * * *"}]
    """

    {:ok, [target]} = Ockam.Healthcheck.Application.parse_config(simple_target)

    ## Set fields
    assert %Target{name: "mytarget", host: "localhost", port: 4000, crontab: "* * * * *"} = target

    ## Default fields
    assert %Target{api_worker: "api", healthcheck_worker: "healthcheck"} = target

    encoded_body = "ZHRlc3Q="

    api_endpoint_target = """
    [{"name": "mytarget",
      "host": "localhost",
      "port": 4000,
      "path": "/",
      "method": "post",
      "body": "#{encoded_body}",
      "api_worker": "api",
      "healthcheck_worker": "healthcheck",
      "crontab": "* * * * *"}]
    """

    {:ok, [target]} = Ockam.Healthcheck.Application.parse_config(api_endpoint_target)

    assert %APIEndpointTarget{
             name: "mytarget",
             host: "localhost",
             port: 4000,
             method: :post,
             body: body,
             api_worker: "api",
             healthcheck_worker: "healthcheck",
             path: "/",
             crontab: "* * * * *"
           } = target

    assert body == Base.decode64!(encoded_body)

    bad_target = "[{\"host\":\"localhost\"}]"

    assert {:error, {:invalid_target, _error}} =
             Ockam.Healthcheck.Application.parse_config(bad_target)

    bad_config = "not json"

    assert {:error, {:invalid_config, _error}} =
             Ockam.Healthcheck.Application.parse_config(bad_config)
  end
end
