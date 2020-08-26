defmodule Ockam.Transport.UDP.MixProject do
  use Mix.Project

  def project do
    [
      app: :ockam_transport_udp,
      version: "0.10.0-dev",
      build_path: "../../_build",
      test_coverage: [output: "../../_build/cover"],
      config_path: "../../config/config.exs",
      deps_path: "../../deps",
      lockfile: "../../mix.lock",
      elixir: "~> 1.10",
      start_permanent: Mix.env() == :prod,
      deps: deps(Mix.env())
    ]
  end

  def application do
    [
      extra_applications: [:logger, :ockam],
      mod: {Ockam.Transport.UDP, []}
    ]
  end

  defp deps(_env) do
    [
      {:ockam, in_umbrella: true}
    ]
  end
end
