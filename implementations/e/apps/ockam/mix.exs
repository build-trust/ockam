defmodule Ockam.MixProject do
  use Mix.Project

  @version "0.10.0-dev"

  @elixir_requirement "~> 1.10"

  @ockam_github_repo "https://github.com/ockam-network/ockam"

  def project do
    [
      app: :ockam,
      version: @version,
      elixir: @elixir_requirement,
      consolidate_protocols: Mix.env() != :test,
      deps: deps(),
      aliases: aliases(),

      # hex
      description: "A collection of tools for building connected systems that you can trust.",
      package: package(),

      # docs
      name: "Ockam",
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
      mod: {Ockam, []},
      extra_applications: [:logger]
    ]
  end

  defp deps do
    [
      # Docs dependencies
      {:ex_doc, "~> 0.22.2", only: :dev, runtime: false}
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
      main: "Ockam",
      source_url: @ockam_github_repo
    ]
  end

  defp aliases do
    []
  end
end
