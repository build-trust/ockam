defmodule Ockly.MixProject do
  use Mix.Project

  def project do
    [
      app: :ockly,
      version: "0.117.0",
      elixir: "~> 1.13",
      start_permanent: Mix.env() == :prod,
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
      {:rustler,
       git: "https://github.com/polvorin/rustler.git",
       branch: "fix_local_crate",
       sparse: "rustler_mix",
       override: true},
      {:rustler_precompiled, "~> 0.7"},
      {:credo, "~> 1.6", only: [:dev, :test], runtime: false},
      {:hkdf_erlang, "~> 1.0.0"}
    ]
  end

  defp aliases do
    [
      credo: "credo --strict",
      "lint.format": "format --check-formatted",
      "lint.credo": "credo --strict",
      lint: ["lint.format", "lint.credo"]
    ]
  end
end
