defmodule Tcbor.MixProject do
  use Mix.Project

  @version "0.1.0"

  @elixir_requirement "~> 1.10"

  def project do
    [
      app: :ockam_typed_cbor,
      version: @version,
      elixir: @elixir_requirement,
      deps: deps(),
      aliases: aliases()
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
      {:cbor, "~> 1.0.0"},
      {:typed_struct, "~> 0.3.0"}
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
