defmodule EarmarkParser.Ast.Renderer.HtmlRenderer do

  import EarmarkParser.Context, only: [prepend: 2]
  import EarmarkParser.Helpers.HtmlParser
  import EarmarkParser.Helpers.AstHelpers, only: [annotate: 2]

  @moduledoc false

  # Structural Renderer for html blocks
  def render_html_block(lines, context, annotation)
  def render_html_block(lines, context, annotation) do
    [tag] = parse_html(lines)
    tag_ = if annotation, do: annotate(tag, annotation), else: tag
    prepend(context, tag_)
  end

  def render_html_oneline([line|_], context, annotation \\ []) do
    [tag|rest] = parse_html([line])
    tag_ = if annotation, do: annotate(tag, annotation), else: tag
    prepend(context, [tag_|rest])
  end
  
  @html_comment_start ~r{\A\s*<!--}
  @html_comment_end ~r{-->.*\z}
  def render_html_comment_line(line) do
    line
    |> String.replace(@html_comment_start, "")
    |> String.replace(@html_comment_end, "")
  end

end
