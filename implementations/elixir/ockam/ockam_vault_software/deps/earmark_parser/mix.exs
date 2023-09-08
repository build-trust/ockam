defmodule EarmarkParser.MixProject do
  use Mix.Project

  @version "1.4.16"
  @url "https://github.com/RobertDober/earmark_parser"

  @deps [
    # {:credo, "~> 0.10", only: [:dev, :test]},
    {:dialyxir, "~> 1.0", only: [:dev, :test], runtime: false},
    {:earmark_ast_dsl, "~> 0.3.0", only: [:test]},
    {:excoveralls, "~> 0.13.3", only: [:test]},
    {:extractly, "~> 0.5.3", only: [:dev]},
    {:floki, "~> 0.21", only: [:dev, :test]}
  ]

  @description """
  Earmark AST the parser and AST Generator for
  Dave Thomas' Earmark.

  The parser generates
  an Abstract Syntax Tree from Markdown.

  The original Earmark will still provide the HTML Transformation and
  the CLI, however its Scanner, Parser and AST Renderer have been
  extracted into this library.
  """

  ############################################################

  def project do
    [
      app: :earmark_parser,
      version: @version,
      elixir: "~> 1.10",
      elixirc_paths: elixirc_paths(Mix.env()),
      deps: @deps,
      description: @description,
      package: package(),
      preferred_cli_env: [
        coveralls: :test,
        "coveralls.detail": :test,
        "coveralls.post": :test,
        "coveralls.html": :test
      ],
      test_coverage: [tool: ExCoveralls],
      aliases: [docs: &build_docs/1]
    ]
  end

  defp package do
    [
      files: [
        "lib",
        "src/*.xrl",
        "src/*.yrl",
        "mix.exs",
        "README.md",
        "RELEASE.md",
        "LICENSE"
      ],
      maintainers: [
        "Robert Dober <robert.dober@gmail.com>"
      ],
      licenses: [
        "Apache-2.0"
      ],
      links: %{
        "Changelog" => "#{@url}/blob/master/RELEASE.md",
        "GitHub" => @url
      }
    ]
  end

  defp elixirc_paths(:test), do: ["lib", "test/support", "dev"]
  defp elixirc_paths(:dev), do: ["lib", "bench", "dev"]
  defp elixirc_paths(_), do: ["lib"]

  @prerequisites """
  run `mix escript.install hex ex_doc` and adjust `PATH` accordingly
  """
  @module "EarmarkParser"
  defp build_docs(_) do
    Mix.Task.run("compile")
    ex_doc = Path.join(Mix.path_for(:escripts), "ex_doc")
    Mix.shell().info("Using escript: #{ex_doc} to build the docs")

    unless File.exists?(ex_doc) do
      raise "cannot build docs because escript for ex_doc is not installed, make sure to \n#{
              @prerequisites
            }"
    end

    args = [@module, @version, Mix.Project.compile_path()]
    opts = ~w[--main #{@module} --source-ref v#{@version} --source-url #{@url}]

    Mix.shell().info("Running: #{ex_doc} #{inspect(args ++ opts)}")
    System.cmd(ex_doc, args ++ opts)
    Mix.shell().info("Docs built successfully")
  end

end

# SPDX-License-Identifier: Apache-2.0
