defmodule Ockam.Umbrella.MixProject do
  use Mix.Project

  @version "0.10.0-dev"

  @elixir_requirement "~> 1.10"

  def project do
    [
      apps_path: "apps",
      version: @version,
      elixir: @elixir_requirement,
      consolidate_protocols: Mix.env() != :test,
      deps: deps(),
      aliases: aliases(),

      # lint
      dialyzer: [flags: ["-Wunmatched_returns", :error_handling, :underspecs]],

      # test
      test_coverage: [output: "_build/cover"]
    ]
  end

  # Dependencies listed here are available only for this umbrella project and
  # cannot be accessed from applications inside the applications folder.
  defp deps do
    [
      {:credo, "~> 1.4", only: [:dev, :test], runtime: false},
      {:dialyxir, "~> 1.0", only: [:dev], runtime: false}
    ]
  end

  defp aliases() do
    [
      docs: "docs --output _build/docs --formatter html",
      test: "test --no-start --cover",
      lint: ["format --check-formatted", "credo --strict"],
      dialyzer: ["dialyzer --format dialyxir"]
    ]
  end
end
