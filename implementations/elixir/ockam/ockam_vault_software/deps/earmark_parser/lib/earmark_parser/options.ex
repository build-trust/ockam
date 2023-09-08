defmodule EarmarkParser.Options do

  use EarmarkParser.Types

  # What we use to render
  defstruct renderer: EarmarkParser.HtmlRenderer,
            # Inline style options
            gfm: true,
            gfm_tables: false,
            breaks: false,
            pedantic: false,
            smartypants: false,
            footnotes: false,
            footnote_offset: 1,
            wikilinks: false,
            parse_inline: true,

            # allow for annotations
            annotations: nil,
            # additional prefies for class of code blocks
            code_class_prefix: nil,

            # Add possibility to specify a timeout for Task.await
            timeout: nil,

            # Very internal—the callback used to perform
            # parallel rendering. Set to &Enum.map/2
            # to keep processing in process and
            # serial
            mapper: &EarmarkParser.pmap/2,
            mapper_with_timeout: &EarmarkParser.pmap/3,

            # Filename and initial line number of the markdown block passed in
            # for meaningful error messages
            file: "<no file>",
            line: 1,
            # [{:error|:warning, lnb, text},...]
            messages: [],
            pure_links: true

  @type t :: %__MODULE__{
        breaks: boolean,
        code_class_prefix: maybe(String.t),
        footnotes: boolean,
        footnote_offset: number,
        gfm: boolean,
        pedantic: boolean,
        pure_links: boolean,
        smartypants: boolean,
        wikilinks: boolean,
        timeout: maybe(number),
        parse_inline: boolean
  }

  @doc ~S"""
  Use normalize before passing it into any API function

        iex(1)> options = normalize(annotations: "%%")
        ...(1)> options.annotations
        ~r{\A(.*)(%%.*)}
  """
  def normalize(options)
  def normalize(%__MODULE__{}=options) do
    case options.annotations do
      %Regex{} -> options
      nil      -> options
      _ -> %{options | annotations: Regex.compile!("\\A(.*)(#{Regex.escape(options.annotations)}.*)")}
    end
  end
  def normalize(options), do: struct(__MODULE__, options) |> normalize()

  @doc false
  # Only here we are aware of which mapper function to use!
  def get_mapper(options) do
    if options.timeout do
      &options.mapper_with_timeout.(&1, &2, options.timeout)
    else
      options.mapper
    end
  end

  @doc false
  def plugin_for_prefix(options, plugin_name) do
    Map.get(options.plugins, plugin_name, false)
  end
end

# SPDX-License-Identifier: Apache-2.0
