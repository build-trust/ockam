defmodule Ockam.Transport.UDP.MixProject do
  use Mix.Project

  @version "0.10.0-dev"

  @elixir_requirement "~> 1.10"

  @ockam_github_repo "https://github.com/ockam-network/ockam"
  @ockam_github_repo_path "implementations/elixir/applications/ockam_transport_udp"

  def project do
    [
      app: :ockam_transport_udp,
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

      # hex
      description: "UDP transport for Ockam.",
      package: package(),

      # docs
      name: "Ockam UDP Transport",
      docs: docs()
    ]
  end

  # mix help compile.app for more
  def application do
    [
      mod: {Ockam.Transport.UDP, []},
      extra_applications: [:logger, :inets, :ockam]
    ]
  end

  defp deps do
    [
      {:ockam, path: "../ockam"},
      {:ex_doc, "~> 0.22.2", only: :dev, runtime: false},
      {:credo, "~> 1.4", only: [:dev, :test], runtime: false},
      {:dialyxir, "~> 1.0", only: [:dev], runtime: false}
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
      main: "Ockam.Transport.UDP",
      source_url_pattern: "#{@ockam_github_repo}/blob/v#{@version}/#{@ockam_github_repo_path}/%{path}#L%{line}"
    ]
  end

  defp aliases do
    [
      docs: "docs --output _build/docs --formatter html",
      test: "test --no-start --cover",
      lint: ["format --check-formatted", "credo --strict"],
      dialyzer: ["dialyzer --format dialyxir"]
    ]
  end
end
