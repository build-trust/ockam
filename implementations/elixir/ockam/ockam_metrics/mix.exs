defmodule Ockam.Metrics.MixProject do
  use Mix.Project

  @version "0.10.1"

  @elixir_requirement "~> 1.10"

  @ockam_github_repo "https://github.com/build-trust/ockam"
  @ockam_github_repo_path "implementations/elixir/ockam/ockam_metrics"

  def project do
    [
      app: :ockam_metrics,
      version: @version,
      elixir: @elixir_requirement,
      consolidate_protocols: Mix.env() != :test,
      elixirc_options: [warnings_as_errors: true],
      deps: deps(),
      aliases: aliases(),

      # lint
      dialyzer: [flags: [:error_handling]],

      # test
      test_coverage: [output: "_build/cover"],
      preferred_cli_env: ["test.cover": :test],

      # hex
      description: "Ockam Metrics",
      package: package(),

      # docs
      name: "Ockam Metrics",
      docs: docs()
    ]
  end

  # mix help compile.app for more
  def application do
    [
      mod: {Ockam.Metrics.Application, []},
      extra_applications: [:logger, :ockam]
    ]
  end

  defp deps do
    [
      {:credo, "~> 1.6", only: [:dev, :test], runtime: false},
      {:dialyxir, "~> 1.1", only: [:dev], runtime: false},
      {:ex_doc, "~> 0.25", only: :dev, runtime: false},
      {:ockam, path: "../ockam", optional: true, runtime: false},
      {:telemetry, "~> 1.0", override: true},
      {:telemetry_poller, "~> 1.0"},
      {:telemetry_metrics, "~> 0.6.1"},
      {:telemetry_metrics_prometheus, "~> 1.1.0"},
      # Needed to avoid conflic on ranch version used by cowboy (telemetry_metrics_prometheus dep)
      {:ranch, "~> 2.1.0", override: true}
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
      main: "Ockam.Metrics",
      source_url_pattern:
        "#{@ockam_github_repo}/blob/v#{@version}/#{@ockam_github_repo_path}/%{path}#L%{line}"
    ]
  end

  defp aliases do
    [
      docs: "docs --output _build/docs --formatter html",
      "lint.format": "format --check-formatted",
      "lint.credo": "credo --strict",
      "lint.dialyzer": "dialyzer --format dialyxir",
      lint: ["lint.format", "lint.credo"],
      test: "test --no-start",
      "test.cover": "test --no-start --cover"
    ]
  end
end
