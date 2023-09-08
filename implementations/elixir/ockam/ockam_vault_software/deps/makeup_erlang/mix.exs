defmodule MakeupErlang.Mixfile do
  use Mix.Project

  @version "0.1.1"
  @url "https://github.com/elixir-makeup/makeup_erlang"

  def project do
    [
      app: :makeup_erlang,
      version: @version,
      elixir: "~> 1.4",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      package: package(),
      name: "Makeup Erlang",
      description: description(),
      aliases: [docs: &build_docs/1]
    ]
  end

  defp description do
    """
    Erlang lexer for the Makeup syntax highlighter.
    """
  end

  defp package do
    [
      name: :makeup_erlang,
      licenses: ["BSD"],
      maintainers: ["Tiago Barroso <tmbb@campus.ul.pt>"],
      links: %{"GitHub" => @url}
    ]
  end

  # Run "mix help compile.app" to learn about applications.
  def application do
    [
      mod: {Makeup.Lexers.ErlangLexer.Application, []},
      extra_applications: [:logger]
    ]
  end

  # Run "mix help deps" to learn about dependencies.
  defp deps do
    [
      {:makeup, "~> 1.0"},
      {:assert_value, "~> 0.9", only: [:dev, :test]}
    ]
  end

  defp build_docs(_) do
    Mix.Task.run("compile")
    ex_doc = Path.join(Mix.path_for(:escripts), "ex_doc")

    unless File.exists?(ex_doc) do
      raise "cannot build docs because escript for ex_doc is not installed"
    end

    args = ["MakeupErlang", @version, Mix.Project.compile_path()]
    opts = ~w[--main Makeup.Lexers.ErlangLexer --source-ref v#{@version} --source-url #{@url}]
    System.cmd(ex_doc, args ++ opts)
    Mix.shell().info("Docs built successfully")
  end
end
