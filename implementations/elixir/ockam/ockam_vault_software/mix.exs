defmodule Ockam.Vault.Software.MixProject do
  use Mix.Project

  @version "0.10.1"

  @elixir_requirement "~> 1.10"

  @ockam_github_repo "https://github.com/build-trust/ockam"
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
      {:ex_doc, "~> 0.25", only: :dev, runtime: false},
      {:credo, "~> 1.6", only: [:dev, :test], runtime: false},
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
      "recompile.native": &recompile_native/1,
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

  defp compile_native(args) do
    case need_recompile_native?() do
      true -> recompile_native(args)
      false -> :ok
    end
  end

  def need_recompile_native?() do
    case {prebuilt_lib_exists?(), test_recompile?()} do
      {true, false} ->
        false

      _ ->
        true
    end
  end

  def test_recompile?() do
    Mix.env() == :test and System.get_env("NO_RECOMPILE_NATIVE") != "true"
  end

  def prebuilt_lib_exists?() do
    case prebuilt_lib_path() do
      {:ok, _path} -> true
      _ -> false
    end
  end

  defp prebuilt_lib_path() do
    with {:ok, subdir} <- os_subdir() do
      case Path.wildcard(Path.join(["priv", subdir, "native", "libockam_elixir_ffi.*"])) do
        [] -> :error
        [_file] -> {:ok, Path.join("priv", subdir)}
      end
    end
  end

  ## NOTE: duplicate in vault_software.ex
  ## we need to run this both in compile-time and in runtime
  defp os_subdir() do
    case {:os.type(), to_string(:erlang.system_info(:system_architecture))} do
      ## Linux libs only built for x86_64
      {{:unix, :linux}, "x86_64" <> _} ->
        {:ok, "linux_x86_64"}

      ## MacOS libs are multi-arch
      {{:unix, :darwin}, "x86_64" <> _} ->
        {:ok, "darwin_universal"}

      {{:unix, :darwin}, "aarch64" <> _} ->
        {:ok, "darwin_universal"}

      _ ->
        :error
    end
  end

  defp recompile_native(args) do
    clean_native(args)
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
        [
          "-S",
          "native",
          "-B",
          native_build_path(),
          "-DBUILD_SHARED_LIBS=ON",
          "-DCMAKE_BUILD_TYPE=Release"
        ],
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
