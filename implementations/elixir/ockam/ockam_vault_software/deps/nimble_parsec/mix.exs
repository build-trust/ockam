defmodule NimbleParsec.MixProject do
  use Mix.Project

  @version "1.1.0"
  @url "https://github.com/dashbitco/nimble_parsec"

  def project do
    [
      app: :nimble_parsec,
      version: @version,
      elixir: "~> 1.6",
      name: "NimbleParsec",
      description: "A simple and fast library for text-based parser combinators",
      aliases: [docs: &build_docs/1],
      package: package()
    ]
  end

  def application do
    []
  end

  defp package do
    %{
      licenses: ["Apache 2"],
      maintainers: ["José Valim"],
      links: %{"GitHub" => "https://github.com/dashbitco/nimble_parsec"}
    }
  end

  defp build_docs(_) do
    Mix.Task.run("compile")
    ex_doc = Path.join(Mix.path_for(:escripts), "ex_doc")

    unless File.exists?(ex_doc) do
      raise "cannot build docs because escript for ex_doc is not installed"
    end

    args = ["NimbleParsec", @version, Mix.Project.compile_path()]
    opts = ~w[--main NimbleParsec --source-ref v#{@version} --source-url #{@url}]
    System.cmd(ex_doc, args ++ opts)
    Mix.shell().info("Docs built successfully")
  end
end
