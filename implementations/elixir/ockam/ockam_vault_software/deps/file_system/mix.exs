defmodule FileSystem.Mixfile do
  use Mix.Project

  def project do
    [ app: :file_system,
      version: "0.2.10",
      elixir: "~> 1.3",
      deps: deps(),
      description: "A file system change watcher wrapper based on [fs](https://github.com/synrc/fs)",
      source_url: "https://github.com/falood/file_system",
      package: package(),
      compilers: [:file_system | Mix.compilers()],
      aliases: ["compile.file_system": &file_system/1],
      docs: [
        extras: ["README.md"],
        main: "readme",
      ]
    ]
  end

  def application do
    [
      applications: [:logger],
    ]
  end

  defp deps do
    [
      { :ex_doc, "~> 0.14", only: :docs },
    ]
  end

  defp file_system(_args) do
    case :os.type() do
      {:unix, :darwin} -> compile_mac()
      _ -> :ok
    end
  end

  defp compile_mac do
    require Logger
    source = "c_src/mac/*.c"
    target = "priv/mac_listener"

    if Mix.Utils.stale?(Path.wildcard(source), [target]) do
      Logger.info "Compiling file system watcher for Mac..."
      cmd = "clang -framework CoreFoundation -framework CoreServices -Wno-deprecated-declarations #{source} -o #{target}"
      if Mix.shell().cmd(cmd) > 0 do
        Logger.error "Could not compile file system watcher for Mac, try to run #{inspect cmd} manually inside the dependency."
      else
        Logger.info "Done."
      end
      :ok
    else
      :noop
    end
  end

  defp package do
    %{ maintainers: ["Xiangrong Hao", "Max Veytsman"],
       files: [
         "lib", "README.md", "mix.exs",
         "c_src/mac/cli.c",
         "c_src/mac/cli.h",
         "c_src/mac/common.h",
         "c_src/mac/compat.c",
         "c_src/mac/compat.h",
         "c_src/mac/main.c",
         "priv/inotifywait.exe",
       ],
       licenses: ["WTFPL"],
       links: %{"Github" => "https://github.com/falood/file_system"}
     }
  end
end
