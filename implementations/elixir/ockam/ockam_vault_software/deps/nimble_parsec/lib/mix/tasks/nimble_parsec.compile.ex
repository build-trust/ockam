defmodule Mix.Tasks.NimbleParsec.Compile do
  @shortdoc "Compiles a parser and injects its content into the parser file"

  @moduledoc ~S"""
  Compiles a parser from a template.

      mix nimble_parsec.compile template.ex.exs

  This task is useful to generate parsers that have no runtime dependency
  on NimbleParsec.

  ## Examples

  Let's define a template file:

      # lib/my_parser.ex.exs
      defmodule MyParser do
        @moduledoc false

        # parsec:MyParser
        import NimbleParsec

        date =
          integer(4)
          |> ignore(string("-"))
          |> integer(2)
          |> ignore(string("-"))
          |> integer(2)

        time =
          integer(2)
          |> ignore(string(":"))
          |> integer(2)
          |> ignore(string(":"))
          |> integer(2)
          |> optional(string("Z"))

        defparsec :datetime, date |> ignore(string("T")) |> concat(time)

        # parsec:MyParser
      end

  After running:

      mix nimble_parsec.compile lib/my_parser.ex.exs

  The following file will be generated:

      # lib/my_parser.ex
      defmodule MyParser do
        @moduledoc false

        def datetime(binary, opts \\ []) do
          ...
        end

        defp datetime__0(...) do
          ...
        end

        ...
      end

  The file will be automatically formatted if using Elixir v1.6+.

  ## Options

    * `-o` - configures the output location. Defaults to the input
      file without its last extension

  """

  use Mix.Task

  @impl true
  def run(args) do
    Mix.Task.reenable("nimble_parsec.compile")
    {opts, files} = OptionParser.parse!(args, strict: [output: :string], aliases: [o: :output])
    Mix.Task.run("compile")

    case files do
      [file] -> compile(file, opts)
      _ -> Mix.raise("Expected a single file to be given to nimble_parsec.compile")
    end
  end

  defp compile(input, opts) do
    output = opts[:output] || Path.rootname(input)
    Mix.shell().info("Generating #{output}")

    {:ok, _} = NimbleParsec.Recorder.start_link([])

    try do
      Code.compiler_options(ignore_module_conflict: true)
      Code.require_file(input)

      input
      |> File.read!()
      |> NimbleParsec.Recorder.replay(input)
      |> write_to_disk(input, output)
    after
      Code.compiler_options(ignore_module_conflict: false)
      NimbleParsec.Recorder.stop()
    end
  end

  defp write_to_disk(contents, input, output) do
    now = DateTime.utc_now() |> Map.put(:microsecond, {0, 0}) |> to_string

    prelude = """
    # Generated from #{input}, do not edit.
    # Generated at #{now}.

    """

    File.write!(output, [prelude | contents])
  end
end
