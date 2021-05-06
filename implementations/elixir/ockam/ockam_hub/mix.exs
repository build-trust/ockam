defmodule Ockam.Hub.MixProject do
  use Mix.Project

  @version "0.10.1"

  @elixir_requirement "~> 1.10"

  @ockam_github_repo "https://github.com/ockam-network/ockam"
  @ockam_github_repo_path "implementations/elixir/ockam/ockam_hub"

  def project do
    [
      app: :ockam_hub,
      version: @version,
      elixir: @elixir_requirement,
      consolidate_protocols: Mix.env() != :test,
      elixirc_options: [warnings_as_errors: true],
      deps: deps(),
      aliases: aliases(),

      # lint
      dialyzer: [flags: ["-Wunmatched_returns", :error_handling, :underspecs]],

      # test
      test_coverage: [output: "_build/cover"],
      preferred_cli_env: ["test.cover": :test],

      # hex
      description: "Ockam Hub.",
      package: package(),

      # docs
      name: "Ockam Hub",
      docs: docs()
    ]
  end

  # mix help compile.app for more
  def application do
    [
      mod: {Ockam.Hub, []},
      extra_applications: [:logger, :ockam]
    ]
  end

  defp deps do
    [
      {:credo, "~> 1.5", only: [:dev, :test], runtime: false},
      {:dialyxir, "~> 1.1", only: [:dev], runtime: false},
      {:ex_doc, "~> 0.23.0", only: :dev, runtime: false},
      {:httpoison, "~> 1.8"},
      {:ockam_vault_software, path: "../ockam_vault_software"},
      {:ockam, path: "../ockam"},
      {:ockam_kafka, path: "../ockam_kafka"},
      {:ranch, "~> 2.0"},
      {:telemetry, "~> 0.4.2"},
      {:telemetry_poller, "~> 0.5.1"},
      {:telemetry_influxdb, "~> 0.2.0"}
    ]
  end

  # used by hex
  defp package do
    [
      links: %{"GitHub" => @ockam_github_repo},
      licenses: ["Apache-2.0"]
    ]
  end

  # used by ex_doc
  defp docs do
    [
      main: "Ockam.Hub",
      source_url_pattern:
        "#{@ockam_github_repo}/blob/v#{@version}/#{@ockam_github_repo_path}/%{path}#L%{line}"
    ]
  end

  defp aliases do
    [
      docs: "docs --output _build/docs --formatter html",
      run: "run --no-halt",
      "lint.format": "format --check-formatted",
      "lint.credo": "credo --strict",
      "lint.dialyzer": "dialyzer --format dialyxir",
      lint: ["lint.format", "lint.credo"],
      "test.cover": "test --no-start --cover"
    ]
  end
end
