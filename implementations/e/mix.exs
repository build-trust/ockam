defmodule Ockam.Umbrella.MixProject do
  use Mix.Project

  def project do
    [
      apps_path: "apps",
      version: "0.10.0-dev",
      start_permanent: Mix.env() == :prod,
      deps: deps(Mix.env()),
      test_coverage: [output: "_build/cover"],
      dialyzer: [flags: ["-Wunmatched_returns", :error_handling, :underspecs]],
      aliases: aliases()
    ]
  end

  # Dependencies listed here are available only for this umbrella project and
  # cannot be accessed from applications inside the applications folder.
  defp deps(:prod) do
    []
  end

  defp deps(_env) do
    deps(:prod) ++
      [
        {:credo, "~> 1.4", only: [:dev, :test], runtime: false},
        {:dialyxir, "~> 1.0", only: [:dev], runtime: false},
        {:ex_doc, "~> 0.22.2", only: [:dev], runtime: false}
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
