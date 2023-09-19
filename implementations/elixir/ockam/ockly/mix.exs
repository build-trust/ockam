defmodule Ockly.MixProject do
  use Mix.Project

  @ockam_release_url "https://github.com/metaclips/ockam/releases/download"
  @version "ockam_v0.86.0"

  def project do
    [
      app: :ockly,
      version: "0.1.0",
      elixir: "~> 1.13",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      aliases: aliases()
    ]
  end

  # Run "mix help compile.app" to learn about applications.
  def application do
    [
      extra_applications: [:logger]
    ]
  end

  # Run "mix help deps" to learn about dependencies.
  defp deps do
    [
      {:rustler,
       git: "https://github.com/polvorin/rustler.git",
       branch: "fix_local_crate",
       sparse: "rustler_mix"},
      {:credo, "~> 1.6", only: [:dev, :test], runtime: false},
      {:hkdf_erlang, "~> 1.0.0"}
    ]
  end

  defp aliases do
    [
      "check.native": &check_native/1,
      compile: ["check.native", "compile"],
      credo: "credo --strict",
      "lint.format": "format --check-formatted",
      "lint.credo": "credo --strict",
      lint: ["lint.format", "lint.credo"]
    ]
  end

  defp check_native(args) do
    case prebuilt_lib_exists?() do
      true -> :ok
      false -> download_native(args)
    end
  end

  defp download_native(_args) do
    with {:ok, lib_name} <- os_specific_lib() do
      download_url = "#{@ockam_release_url}/#{@version}/#{lib_name}"

      ## To download files we need inets and ssl
      :inets.start()
      :ssl.start()

      dest_file = prebuilt_lib_path()

      dest_dir = Path.dirname(dest_file)

      File.mkdir_p!(dest_dir)

      IO.puts("Downloading lib from #{download_url} to #{dest_file}")
      File.rm_rf(dest_file)

      download_result =
        :httpc.request(
          :get,
          {to_charlist(download_url), []},
          # We should ensure we test this TLS version when
          # we update OTP.
          [{:ssl, [{:versions, [:"tlsv1.2"]}]}],
          stream: to_charlist(dest_file)
        )

      case download_result do
        {:ok, :saved_to_file} -> IO.puts("Download OK")
        {:ok, other} -> IO.puts("Download error: #{inspect(other)}")
        other -> IO.puts("Download error: #{inspect(other)}")
      end
    end
  end

  defp prebuilt_lib_exists?() do
    File.exists?(prebuilt_lib_path())
  end

  defp prebuilt_lib_path() do
    Path.join(["priv", "native", "libockly.so"])
  end

  defp os_specific_lib() do
    case {:os.type(), to_string(:erlang.system_info(:system_architecture))} do
      ## Linux libs only built for GNU
      {{:unix, :linux}, "x86_64" <> type} ->
        if String.ends_with?(type, "gnu") do
          {:ok, "linux_x86_libockly.so"}
        else
          :error
        end

      {{:unix, :linux}, "aarch64" <> _type} ->
        :error

      ## MacOS libs are multi-arch
      {{:unix, :darwin}, "x86_64" <> _} ->
        {:ok, "darwin_universal_libockly.so"}

      {{:unix, :darwin}, "aarch64" <> _} ->
        {:ok, "darwin_universal_libockly.so"}

      _ ->
        :error
    end
  end
end
