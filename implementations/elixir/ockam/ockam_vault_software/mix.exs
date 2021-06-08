defmodule Ockam.Vault.Software.MixProject do
  use Mix.Project

  @version "0.10.1"

  @elixir_requirement "~> 1.10"

  @ockam_github_repo "https://github.com/ockam-network/ockam"
  @ockam_github_repo_path "implementations/elixir/ockam/ockam_vault_software"

  def project do
    [
      app: :ockam_vault_software,
      version: @version,
      elixir: @elixir_requirement,
      consolidate_protocols: Mix.env() != :test,
      elixirc_options: [warnings_as_errors: true],
      deps: deps(),
      aliases: aliases(),

      # lint
      dialyzer: [flags: ["-Wunmatched_returns", :underspecs]],

      # test
      test_coverage: [output: "_build/cover"],
      preferred_cli_env: ["test.cover": :test],

      # hex
      description: "A software implementation of the ockam vault behaviour.",
      package: package(),

      # docs
      name: "Ockam Vault Software",
      docs: docs()
    ]
  end

  # mix help compile.app for more
  def application do
    [
      mod: {Ockam.Vault.Software, []},
      extra_applications: []
    ]
  end

  defp deps do
    [
      {:ex_doc, "~> 0.24.0", only: :dev, runtime: false},
      {:credo, "~> 1.5", only: [:dev, :test], runtime: false},
      {:dialyxir, "~> 1.1", only: [:dev], runtime: false}
    ]
  end

  # used by hex
  defp package do
    [
      links: %{"GitHub" => @ockam_github_repo},
      licenses: ["Apache-2.0"]
    ]
  end

  # used by ex_doc
  defp docs do
    [
      main: "Ockam.Vault.Software",
      source_url_pattern:
        "#{@ockam_github_repo}/blob/v#{@version}/#{@ockam_github_repo_path}/%{path}#L%{line}"
    ]
  end

  defp aliases do
    [
      "compile.native": &compile_native/1,
      "clean.native": &clean_native/1,
      compile: ["compile.native", "compile"],
      clean: ["clean", "clean.native"],
      docs: "docs --output _build/docs --formatter html",
      "test.cover": "test --no-start --cover",
      "lint.format": "format --check-formatted",
      "lint.credo": "credo --strict",
      "lint.dialyzer": "dialyzer --format dialyxir",
      lint: ["lint.format", "lint.credo"]
    ]
  end

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

    Enum.each(["dylib", "so"], fn extension ->
      Path.join([native_build_path(), "**", "*.#{extension}"])
      |> Path.wildcard()
      |> Enum.each(fn lib ->
        filename = Path.basename(lib, ".#{extension}")
        destination = Path.join(priv_path, "#{filename}.so")
        File.cp!(lib, destination)
      end)
    end)

    :ok
  end
end
