defmodule Erlex.MixProject do
  use Mix.Project

  def project do
    [
      app: :erlex,
      version: "0.2.6",
      elixir: "~> 1.6",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      description: description(),
      package: package(),
      docs: docs(),
      test_coverage: [tool: ExCoveralls],
      dialyzer: [
        # plt_core_path: ".",
        plt_add_apps: [:mix, :erts, :kernel, :stdlib],
        flags: ["-Wunmatched_returns", "-Werror_handling", "-Wrace_conditions", "-Wno_opaque"],
        ignore_warnings: "dialyzer.ignore_warnings.exs"
      ],
      preferred_cli_env: [
        coveralls: :test,
        "coveralls.detail": :test,
        "coveralls.post": :test,
        "coveralls.html": :test
      ]
    ]
  end

  def application do
    [
      extra_applications: [:logger]
    ]
  end

  defp deps do
    [
      {:credo, "~> 0.9", only: [:dev, :test], runtime: false},
      {:ex_doc, ">= 0.0.0", only: :dev, runtime: false},
      {:dialyxir, "~> 1.0.0-rc.3", only: :dev, runtime: false},
      {:excoveralls, "~> 0.8", only: :test}
    ]
  end

  defp description do
    """
    Convert Erlang style structs and error messages to equivalent Elixir.
    """
  end

  defp docs() do
    [
      main: "readme",
      source_url: "https://github.com/asummers/erlex",
      homepage_url: "https://github.com/asummers/erlex",
      extras: ["README.md", "CHANGELOG.md"]
    ]
  end

  defp package do
    [
      files: [
        "lib",
        "mix.exs",
        "README.md",
        "LICENSE.md",
        "src/lexer.xrl",
        "src/parser.yrl"
      ],
      maintainers: ["Andrew Summers"],
      licenses: ["Apache 2.0"],
      links: %{"GitHub" => "https://github.com/asummers/erlex"}
    ]
  end
end
