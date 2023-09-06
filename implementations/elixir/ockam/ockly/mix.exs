defmodule Ockly.MixProject do
  use Mix.Project

  def project do
    [
      app: :ockly,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps()
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
	{:rustler, git: "https://github.com/polvorin/rustler.git", branch: "fix_local_crate", sparse: "rustler_mix"},
	{:hkdf_erlang, "~> 1.0.0"},
    ]
  end
end
