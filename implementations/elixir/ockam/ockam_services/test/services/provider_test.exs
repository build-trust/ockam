defmodule Test.Services.ProviderTest do
  use ExUnit.Case

  alias Ockam.Services.Provider

  test "provider parses json config" do
    json = Jason.encode!(%{my_service: ["a", "b"], other_service: ["arg"]})
    Application.put_env(:ockam_services, :services_config_source, "json")
    Application.put_env(:ockam_services, :services_json, json)

    assert [my_service: ["a", "b"], other_service: ["arg"]] = Provider.get_configured_services()
  end

  test "provider parses list" do
    list = "my_service, other_service"

    Application.put_env(:ockam_services, :services_config_source, "list")
    Application.put_env(:ockam_services, :services_list, list)

    assert [my_service: [], other_service: []] = Provider.get_configured_services()
  end

  test "provider parses file" do
    json = Jason.encode!(%{my_service: ["a", "b"], other_service: ["arg"]})
    filename = "service_config"
    dir = System.tmp_dir!()
    tmp_file = Path.join(dir, filename)

    on_exit(fn ->
      File.rm_rf(tmp_file)
    end)

    File.rm_rf(tmp_file)
    File.write(tmp_file, json)

    Application.put_env(:ockam_services, :services_config_source, "file")
    Application.put_env(:ockam_services, :services_file, tmp_file)

    assert [my_service: ["a", "b"], other_service: ["arg"]] = Provider.get_configured_services()
  end
end
