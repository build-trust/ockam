defmodule Ockam.Node.Web.MixProject do
  use Mix.Project

  @version "0.10.1"

  @elixir_requirement "~> 1.10"

  @ockam_github_repo "https://github.com/ockam-network/ockam"
  @ockam_github_repo_path "implementations/elixir/ockam/ockam_node_web"

  def project do
    [
      app: :ockam_node_web,
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
      description: "A web interface for an ockam node",
      package: package(),

      # docs
      name: "Ockam Node Web",
      docs: docs()
    ]
  end

  # mix help compile.app for more
  def application do
    [
      mod: {Ockam.Node.Web, []},
      extra_applications: [:logger]
    ]
  end

  defp deps do
    [
      {:ranch, "~> 2.1", override: true},
      {:cowboy, "~> 2.9"},
      {:plug, "~> 1.12"},
      {:plug_cowboy, "~> 2.5"},
      {:jason, "~> 1.2"},
      {:ockam, path: "../ockam"},
      {:ex_doc, "~> 0.25", only: :dev, runtime: false},
      {:credo, "~> 1.6", only: [:dev, :test], runtime: false},
      {:dialyxir, "~> 1.1", only: [:dev], runtime: false}
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
      main: "Ockam.Node.Web",
      source_url_pattern:
        "#{@ockam_github_repo}/blob/v#{@version}/#{@ockam_github_repo_path}/%{path}#L%{line}"
    ]
  end

  defp aliases do
    [
      docs: "docs --output _build/docs --formatter html",
      "test.cover": "test --no-start --cover",
      "lint.format": "format --check-formatted",
      "lint.credo": "credo --strict",
      "lint.dialyzer": "dialyzer --format dialyxir",
      lint: ["lint.format", "lint.credo"]
    ]
  end
end
