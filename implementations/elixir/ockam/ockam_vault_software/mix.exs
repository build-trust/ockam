defmodule Ockam.Vault.Software.MixProject do
  use Mix.Project

  @version "0.70.0"

  @elixir_requirement "~> 1.12"

  @ockam_release_url "https://github.com/build-trust/ockam/releases"
  @download_libs [
    {"ockam.linux_elixir_ffi.so", ["linux_x86_64_gnu", "native", "libockam_elixir_ffi.so"]},
    {"ockam.darwin_universal_elixir_ffi.so",
     ["darwin_universal", "native", "libockam_elixir_ffi.so"]}
  ]

  @ockam_github_repo "https://github.com/build-trust/ockam"
  @ockam_github_repo_path "implementations/elixir/ockam/ockam_vault_software"

  def project do
    [
      app: :ockam_vault_software,
      version: package_version(),
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
      licenses: ["Apache-2.0"],
      source_url: @ockam_github_repo
    ]
  end

  # used by ex_doc
  defp docs do
    [
      main: "Ockam.Vault.Software",
      source_url_pattern:
        "#{@ockam_github_repo}/blob/v#{package_version()}/#{@ockam_github_repo_path}/%{path}#L%{line}"
    ]
  end

  defp aliases do
    [
      "check.native": &check_native/1,
      "download.native": &download_native/1,
      "recompile.native": &recompile_native/1,
      "clean.native": &clean_native/1,
      "hex.build": ["download.native --version=#{package_version()}", "hex.build"],
      "hex.publish": ["download.native --version=#{package_version()}", "hex.publish"],
      compile: ["check.native", "compile"],
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

  defp check_native(args) do
    case test_recompile?() do
      true ->
        recompile_native(args)

      _ ->
        case prebuilt_lib_exists?() do
          true -> :ok
          false -> download_native(args)
        end

        ## Check again if download failed or file is missing
        case prebuilt_lib_exists?() do
          true ->
            :ok

          false ->
            IO.puts("Could not download prebuilt lib. Recompiling.")
            recompile_native(args)
        end
    end
  end

  defp download_native(args) do
    version_path =
      case ockam_version(args) do
        "latest" -> "/latest/download"
        other -> "/download/ockam_" <> other
      end

    base_url = @ockam_release_url <> version_path

    ## To donwload files we need inets and ssl
    :inets.start()
    :ssl.start()

    Enum.each(@download_libs, fn {from, to} ->
      download_url = base_url <> "/" <> from
      dest_path = ["priv" | to]

      dest_file = Path.join(dest_path)
      dest_dir = Path.join(Enum.take(dest_path, length(dest_path) - 1))

      File.mkdir_p!(dest_dir)

      IO.puts("Downloading lib from #{download_url} to #{dest_file}")
      File.rm_rf(dest_file)

      {:ok, :saved_to_file} =
        :httpc.request(
          :get,
          {to_charlist(download_url), []},
          [],
          stream: to_charlist(dest_file)
        )
    end)
  end

  defp package_version() do
    case System.get_env("VERSION") do
      nil -> @version
      version -> version
    end
  end

  defp ockam_version(args) do
    case OptionParser.parse(args, switches: [version: :string]) do
      {[version: version], _, _} ->
        "v" <> version

      _ ->
        case Mix.env() do
          :dev -> "latest"
          :prod -> "v" <> package_version()
        end
    end
  end

  defp test_recompile?() do
    Mix.env() == :test and System.get_env("NO_RECOMPILE_NATIVE") != "true"
  end

  defp prebuilt_lib_exists?() do
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
      ## Linux libs only built for GNU
      {{:unix, :linux}, "x86_64" <> type} ->
        if String.ends_with?(type, "gnu") do
          {:ok, "linux_x86_64_gnu"}
        else
          :error
        end

      {{:unix, :linux}, "aarch64" <> type} ->
        if String.ends_with?(type, "gnu") do
          {:ok, "linux_aarch64_gnu"}
        else
          :error
        end

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
