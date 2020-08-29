defmodule Ockam.Node.Web.MixProject do
  use Mix.Project

  @version "0.10.0-dev"

  @elixir_requirement "~> 1.10"

  @ockam_github_repo "https://github.com/ockam-network/ockam"

  def project do
    [
      app: :ockam_node_web,
      version: @version,
      elixir: @elixir_requirement,
      consolidate_protocols: Mix.env() != :test,
      compilers: [:phoenix] ++ Mix.compilers(),
      deps: deps(),
      aliases: aliases(),

      # hex
      description: "A web interface for an ockam node",
      package: package(),

      # docs
      name: "Ockam Node Web",
      docs: docs()
    ] ++ project(in_umbrella: Mix.Project.umbrella?())
  end

  defp project(in_umbrella: true) do
    [
      test_coverage: [output: "../../_build/cover"]
    ]
  end

  defp project(in_umbrella: false), do: []

  # mix help compile.app for more
  def application do
    [
      mod: {Ockam.Node.Web, []},
      extra_applications: [:logger, :ockam, :phoenix]
    ]
  end

  defp deps do
    [
      {:phoenix, "~> 1.5.4"},
      {:phoenix_html, "~> 2.11"},
      {:jason, "~> 1.0"},
      {:plug_cowboy, "~> 2.0"},

      # Docs dependencies
      {:ex_doc, "~> 0.22.2", only: :dev, runtime: false}
    ] ++ deps(in_umbrella: Mix.Project.umbrella?())
  end

  defp deps(in_umbrella: true) do
    [
      {:ockam, in_umbrella: true}
    ]
  end

  defp deps(in_umbrella: false) do
    [
      {:ockam, "~> 0.10.0-dev"}
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
      source_url: @ockam_github_repo
    ]
  end

  defp aliases do
    []
  end
end
