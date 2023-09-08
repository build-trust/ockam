defmodule Makeup.Formatters.HTML.HTMLFormatter do
  @moduledoc """
  A turn the list of tokens into a 
  """

  @group_highlight_js "lib/makeup/formatters/html/scripts/group_highlighter_javascript.js" |> File.read!

  defp render_token(escaped_value, css_class, meta, highlight_tag) do
    group_id = meta[:group_id]
    selectable = Map.get(meta, :selectable, [])

    classes = [
      css_class || [],
      if selectable == false do " unselectable" else [] end
    ]

    [
      "<",
      highlight_tag,
      ~S( class="),
      classes,
      ~S("),
      if group_id do [~S( data-group-id="), group_id, ~S(")] else [] end,
      ">",
      escaped_value,
      "</",
      highlight_tag,
      ">",
    ]
  end

  @doc """
  format a single token into an iolist
  """
  def format_token({tag, meta, value}, highlight_tag) do
    escaped_value = escape(value)
    css_class = Makeup.Token.Utils.css_class_for_token_type(tag)
    render_token(escaped_value, css_class, meta, highlight_tag)
  end

  defp escape_for(?&), do: "&amp;"

  defp escape_for(?<), do: "&lt;"

  defp escape_for(?>), do: "&gt;"

  defp escape_for(?"), do: "&quot;"

  defp escape_for(?'), do: "&#39;"

  defp escape_for(c) when is_integer(c) and c <= 127, do: c

  defp escape_for(c) when is_integer(c) and c >= 128, do: << c :: utf8 >>

  defp escape_for(string) when is_binary(string) do
    string
    |> to_charlist()
    |> Enum.map(&escape_for/1)
  end

  defp escape(iodata) when is_list(iodata) do
    iodata
    |> :lists.flatten()
    |> Enum.map(&escape_for/1)
  end

  defp escape(other) when is_binary(other) do
    escape_for(other)
  end

  defp escape(c) when is_integer(c) do
    #
    [escape_for(c)]
  end

  defp escape(other) do
    raise "Found `#{inspect(other)}` inside what should be an iolist"
  end

  @doc """
  Turns a list of tokens into an iolist which represents an HTML fragment.
  This fragment can be embedded directly into an HTML document.
  """
  def format_inner_as_iolist(tokens, opts) do
    highlight_tag = Keyword.get(opts, :highlight_tag, "span")
    Enum.map(tokens, &format_token(&1, highlight_tag))
  end

  @doc """
  Turns a list of tokens into an HTML fragment.
  This fragment can be embedded directly into an HTML document.
  """
  def format_inner_as_binary(tokens, opts) do
    tokens
    |> format_inner_as_iolist(opts)
    |> IO.iodata_to_binary
  end

  @doc """
  Turns a list of tokens into an iolist which represents an HTML fragment.
  This fragment can be embedded directly into an HTML document.
  """
  def format_as_iolist(tokens, opts \\ []) do
    css_class = Keyword.get(opts, :css_class, "highlight")
    inner = format_inner_as_iolist(tokens, opts)

    [
      ~S(<pre class="),
      css_class,
      ~S("><code>),
      inner,
      ~S(</code></pre>)
    ]
  end

  @doc """
  Turns a list of tokens into an HTML fragment.
  This fragment can be embedded directly into an HTML document.
  """
  def format_as_binary(tokens, opts \\ []) do
    tokens
    |> format_as_iolist(opts)
    |> IO.iodata_to_binary
  end

  @doc """
  Return the CSS stylesheet for a given style.
  """
  def stylesheet(style, css_class \\ "highlight") do
    Makeup.Styles.HTML.Style.stylesheet(style, css_class)
  end

  @doc """
  Return a Javascript snippet to highlight code on mouseover.
  This is "raw" javascript, and for inclusion in an HTML file
  it must be wrapped in a `<script>` tag.
  """
  def group_highlighter_javascript() do
    @group_highlight_js
  end
end
