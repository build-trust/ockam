defmodule Ockam.MixProject do
  use Mix.Project

  def project do
    [
      app: :ockam,
      version: "0.10.0-dev",
      build_path: "../../_build",
      test_coverage: [output: "../../_build/cover"],
      config_path: "../../configuration/config.exs",
      deps_path: "../../deps",
      lockfile: "../../mix.lock",
      elixir: "~> 1.10",
      start_permanent: Mix.env() == :prod,
      deps: deps(Mix.env()),
      aliases: aliases()
    ]
  end

  def application do
    [
      extra_applications: [:logger],
      mod: {Ockam, []}
    ]
  end

  defp deps(:prod) do
    []
  end

  defp deps(_) do
    deps(:prod) ++ [
      {:ex_doc, "~> 0.22.2", only: [:dev], runtime: false}
    ]
  end

  defp aliases() do
    [
      docs: "docs --output ../../_build/docs",
    ]
  end
end
