defmodule Ockam.MixProject do
  use Mix.Project

  def project do
    [
      app: :ockam,
      version: "0.9.0",
      elixir: "~> 1.9",
      start_permanent: Mix.env() == :prod,
      deps: deps(Mix.env()),
      elixirc_paths: elixirc_paths(Mix.env()),
      test_coverage: [output: "_build/cover"],
      dialyzer: [flags: ["-Wunmatched_returns", :error_handling, :underspecs]],
      aliases: [
        compile: [&compile_native/1, "compile"],
        clean: ["clean", &clean_native/1],
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

  defp elixirc_paths(:test), do: ["lib", "test/support"]
  defp elixirc_paths(_), do: ["lib"]

  defp native_build_path(), do: Path.join([Mix.Project.build_path(), "native"])

  defp native_priv_path() do
    Path.join([Mix.Project.app_path(), "priv", "native"])
  end

  defp compile_native(_args) do
    :ok = cmake_generate()
    :ok = cmake_build()
    :ok = copy_to_priv()
    :ok
  end

  defp clean_native(_) do
    File.rm_rf!(native_build_path())
    File.rm_rf!(native_priv_path())
  end

  defp cmake_generate() do
    {_, 0} =
      System.cmd(
        "cmake",
        ["-S", "native", "-B", native_build_path(), "-DBUILD_SHARED_LIBS=ON"],
        into: IO.stream(:stdio, :line),
        env: [{"ERL_INCLUDE_DIR", erl_include_dir()}]
      )
    :ok
  end

  defp cmake_build() do
    {_, 0} =
      System.cmd(
        "cmake",
        ["--build", native_build_path()],
        into: IO.stream(:stdio, :line),
        env: [{"ERL_INCLUDE_DIR", erl_include_dir()}]
      )
    :ok
  end

  defp erl_include_dir() do
    [:code.root_dir(), Enum.concat('erts-', :erlang.system_info(:version)), 'include']
    |> Path.join()
    |> to_string
  end

  defp copy_to_priv() do
    priv_path = native_priv_path()
    File.mkdir_p!(priv_path)

    # this likely only works on macos,
    # TODO(mrinal): make this work on all operating systems
    Path.join([native_build_path(), "**", "*.dylib"])
    |> Path.wildcard()
    |> Enum.each(fn(lib) ->
      filename = Path.basename(lib, ".dylib")
      destination = Path.join(priv_path, "#{filename}.so")
      File.cp!(lib, destination)
    end)

    :ok
  end

end
