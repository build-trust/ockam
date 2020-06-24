defmodule Ockam.MixProject do
  use Mix.Project

  def project do
    [
      app: :ockam,
      version: "0.8.0",
      elixir: "~> 1.9",
      start_permanent: Mix.env() == :prod,
      deps: deps(Mix.env()),
      elixirc_paths: elixirc_paths(Mix.env()),
      rustler_crates: rustler_crates(Mix.env()),
      compilers: [:rustler] ++ Mix.compilers(),
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
      {:rustler, "~> 0.21"},
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

  defp rustler_crates(env) do
    cwd = File.cwd!()
    ockam_root = Path.join([cwd, "..", ".."])

    [
      ockam_nif: [
        path: "priv/ockam_nif",
        mode: rust_mode(env),
        env: [
          {"OCKAM_ROOT", Path.expand(ockam_root)}
        ]
      ]
    ]
  end

  defp rust_mode(:prod), do: :release
  defp rust_mode(_), do: :debug

  defp elixirc_paths(:test), do: ["lib", "test/support"]
  defp elixirc_paths(_), do: ["lib"]
end
