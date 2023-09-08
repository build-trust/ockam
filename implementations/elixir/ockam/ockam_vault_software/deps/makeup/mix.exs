defmodule Makeup.Mixfile do
  use Mix.Project

  @version "1.0.5"
  @url "https://github.com/tmbb/makeup"

  def project do
    [
      app: :makeup,
      version: @version,
      elixir: "~> 1.4",
      elixirc_paths: compiler_paths(Mix.env()),
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      # Docs
      name: "Makeup",
      source_url: @url,
      homepage_url: @url,
      aliases: aliases(),
      docs: [
        # The main page in the docs
        main: "readme",
        extras: [
          "README.md"
        ]
      ],
      # Package
      package: package(),
      description: description()
    ]
  end

  defp description do
    """
    Syntax highlighter for source code in the style of Pygments.
    """
  end

  def compiler_paths(:dev), do: ["test/helpers"] ++ compiler_paths(:prod)
  def compiler_paths(:test), do: ["test/helpers"] ++ compiler_paths(:prod)
  def compiler_paths(_), do: ["lib"]

  defp aliases do
    [
      docs: &build_docs/1
    ]
  end

  defp package do
    [
      name: :makeup,
      licenses: ["BSD"],
      maintainers: ["Tiago Barroso <tmbb@campus.ul.pt>"],
      links: %{"GitHub" => @url}
    ]
  end

  # Run "mix help compile.app" to learn about applications.
  def application do
    [
      extra_applications: [:eex],
      mod: {Makeup.Application, []}
    ]
  end

  # Run "mix help deps" to learn about dependencies.
  defp deps do
    [
      {:nimble_parsec, "~> 0.5 or ~> 1.0"},
      {:stream_data, "~> 0.4.2", only: [:dev, :test]}
    ]
  end

  defp build_docs(_) do
    Mix.Task.run("compile")
    ex_doc = Path.join(Mix.path_for(:escripts), "ex_doc")

    unless File.exists?(ex_doc) do
      raise "cannot build docs because escript for ex_doc is not installed"
    end

    args = ["Makeup", @version, Mix.Project.compile_path()]
    opts = ~w[--main Makeup --source-ref v#{@version} --source-url #{@url}]
    System.cmd(ex_doc, args ++ opts)
    Mix.shell().info("Docs built successfully")
  end
end
