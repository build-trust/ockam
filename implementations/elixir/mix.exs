defmodule Ockam.MixProject do
  use Mix.Project

  def project do
    [
      app: :ockam,
      version: "0.9.0",
      elixir: "~> 1.9",
      start_permanent: Mix.env() == :prod,
      deps: deps(Mix.env()),
      elixirc_paths: elixirc_paths(Mix.env()),
      test_coverage: [output: "_build/cover"],
      dialyzer: [flags: ["-Wunmatched_returns", :error_handling, :underspecs]],
      aliases: [
        docs: "docs --output _build/docs",
        test: "test --no-start --cover",
        lint: ["credo --strict --format oneline", "format --check-formatted"]
      ]
    ]
  end

  def application do
    [
      extra_applications: [:logger, :inets, :ranch],
      mod: {Ockam, []}
    ]
  end

  def deps(:prod) do
    [
      {:ranch, "~> 2.0.0-rc.2"},
      {:gen_state_machine, "~> 2.1"}
    ]
  end

  def deps(_) do
    deps(:prod) ++
      [
        {:ex_doc, "~> 0.21", only: [:dev], runtime: false},
        {:credo, "~> 1.4", only: [:dev, :test], runtime: false},
        {:dialyxir, "~> 1.0", only: [:dev], runtime: false}
      ]
  end

  defp elixirc_paths(:test), do: ["lib", "test/support"]
  defp elixirc_paths(_), do: ["lib"]
end
