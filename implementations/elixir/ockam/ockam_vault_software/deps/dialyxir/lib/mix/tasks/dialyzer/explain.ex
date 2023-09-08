defmodule Mix.Tasks.Dialyzer.Explain do
  @shortdoc "Display information about Dialyzer warnings."

  @moduledoc """
  This task provides background information about Dialyzer warnings.
  If invoked without any arguments it will list all warning atoms.
  When invoked with the name of a particular warning, it will display
  information regarding it.

  ## Command line options

  * `[warning]`       - display information regarding warning

  ```
  mix dialyzer.explain pattern_match
  ```
  """
  use Mix.Task
  alias Dialyxir.Output

  def run(args) do
    case OptionParser.parse(args, strict: []) do
      {_, [warning], _} ->
        warning |> explanation_text() |> Output.info()

      {_, [], _} ->
        list_warnings() |> Output.info()

      _ ->
        Mix.Task.run("help", ["dialyzer.explain"])
    end
  end

  defp explanation_text(warning_name) do
    warning = String.to_atom(warning_name)

    case Map.get(Dialyxir.Warnings.warnings(), warning) do
      nil ->
        "Unknown warning named: #{warning_name}"

      module ->
        module.explain()
    end
  end

  defp list_warnings do
    warnings =
      Dialyxir.Warnings.warnings()
      |> Map.keys()
      |> Enum.sort()
      |> Enum.map_join("\n", &Atom.to_string/1)

    [
      """
      Explain warning with mix dialyzer.explain <warning>

      #{warnings}
      """
    ]
  end
end
