defmodule OckamKinesis.MixProject do
  use Mix.Project

  def project do
    [
      app: :ockam_kinesis,
      version: "0.1.0",
      elixir: "~> 1.13",
      start_permanent: Mix.env() == :prod,
      elixirc_options: [warnings_as_errors: true],
      deps: deps(),
      elixirc_paths: elixirc_paths(Mix.env())
    ]
  end

  def elixirc_paths(:test), do: ["lib", "test/support"]
  def elixirc_paths(_), do: ["lib"]

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
      {:ex_aws, "~> 2.3"},
      {:ex_aws_kinesis,
       git: "https://github.com/ex-aws/ex_aws_kinesis.git",
       ref: "2b1a13967e8dd3c05a8c275852923eb8037da85d"},
      {:ex_doc, "~> 0.25", only: :dev, runtime: false},
      {:dialyxir, "~> 1.1", only: [:dev], runtime: false},
      {:hackney, "~> 1.18"},
      {:jason, "~> 1.3"},
      {:ockam_services, path: "../ockam_services"},
      {:mox, "~> 1.0.2", only: :test}
    ]
  end
end
