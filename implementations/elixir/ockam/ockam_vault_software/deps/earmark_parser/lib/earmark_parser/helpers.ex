defmodule EarmarkParser.Helpers do

  @moduledoc false
  @doc """
  Expand tabs to multiples of 4 columns
  """
  def expand_tabs(line) do
    Regex.replace(~r{(.*?)\t}, line, &expander/2)
  end

  @trailing_ial_rgx ~r< (\s* \S .*?) {: \s* ([^}]+) \s* } \s* \z>x
  @doc ~S"""
  Returns a tuple containing a potentially present IAL and the line w/o the IAL

      iex(1)> extract_ial("# A headline")
      {nil, "# A headline"}

      iex(2)> extract_ial("# A classy headline{:.classy}")
      {".classy", "# A classy headline"}

  An IAL line, remains an IAL line though

      iex(3)> extract_ial("{:.line-ial}")
      {nil, "{:.line-ial}"}
  """
  def extract_ial(line) do
    case Regex.run(@trailing_ial_rgx, line) do
      nil -> {nil, line}
      [_, line_, ial] -> {ial, line_}
    end
  end

  defp expander(_, leader) do
    extra = 4 - rem(String.length(leader), 4)
    leader <> pad(extra)
  end

  @doc """
  Remove newlines at end of line and optionally annotations
  """
  # def remove_line_ending(line, annotation \\ nil)
  def remove_line_ending(line, nil) do
    _trim_line({line, nil})
  end
  def remove_line_ending(line, annotation) do
    case Regex.run(annotation, line) do
      nil -> _trim_line({line, nil})
      match -> match |> tl() |> List.to_tuple |> _trim_line()
    end
  end

  defp _trim_line({line, annot}), do: {line |> String.trim_trailing("\n") |> String.trim_trailing("\r"), annot}

  defp pad(1), do: " "
  defp pad(2), do: "  "
  defp pad(3), do: "   "
  defp pad(4), do: "    "

  @doc """
  `Regex.replace` with the arguments in the correct order
  """

  def replace(text, regex, replacement, options \\ []) do
    Regex.replace(regex, text, replacement, options)
  end

  @doc """
  Replace <, >, and quotes with the corresponding entities. If
  `encode` is true, convert ampersands, too, otherwise only
   convert non-entity ampersands.
  """

  @amp_rgx ~r{&(?!#?\w+;)}

  def escape(html), do: _escape(Regex.replace(@amp_rgx, html, "&amp;"))


  defp _escape(html) do
    html
    |> String.replace("<", "&lt;")
    |> String.replace(">", "&gt;")
    |> String.replace("\"", "&quot;")
    |> String.replace("'", "&#39;")
  end
end

# SPDX-License-Identifier: Apache-2.0
