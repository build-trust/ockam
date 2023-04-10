defmodule OckamKafka.MixProject do
  use Mix.Project

  def project do
    [
      app: :ockam_kafka,
      version: "0.1.0",
      elixir: "~> 1.11",
      start_permanent: Mix.env() == :prod,
      elixirc_options: [warnings_as_errors: true],
      deps: deps(),
      aliases: aliases(),

      # lint
      dialyzer: [flags: [:error_handling, :underspecs]],

      # test
      test_coverage: [output: "_build/cover"],
      preferred_cli_env: ["test.cover": :test]
    ]
  end

  # Run "mix help compile.app" to learn about applications.
  def application do
    [
      extra_applications: [:logger]
    ]
  end

  # Run "mix help deps" to learn about dependencies.
  defp deps do
    [
      {:credo, "~> 1.6", only: [:dev, :test], runtime: false},
      {:dialyxir, "~> 1.1", only: [:dev], runtime: false},
      {:ex_doc, "~> 0.25", only: :dev, runtime: false},
      {:ockam_services, path: "../ockam_services"},
      {:brod,
       git: "https://github.com/hairyhum/brod.git", branch: "kpro-connection-timeout-3.15"},
      {:snappyer, "~> 1.2", override: true}
      # {:brod, "~> 3.14.0"},
    ]
  end

  defp aliases do
    [
      credo: "credo --strict",
      docs: "docs --output _build/docs --formatter html",
      "test.cover": "test --cover",
      "lint.format": "format --check-formatted",
      "lint.credo": "credo --strict",
      "lint.dialyzer": "dialyzer --format dialyxir",
      lint: ["lint.format", "lint.credo"]
    ]
  end
end
