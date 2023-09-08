defmodule EarmarkParser.AstRenderer do
  alias EarmarkParser.Block
  alias EarmarkParser.Context
  alias EarmarkParser.Options

  import Context, only: [clear_value: 1, modify_value: 2, prepend: 2, prepend: 3]
  import EarmarkParser.Ast.Emitter
  import EarmarkParser.Ast.Inline, only: [convert: 3]
  import EarmarkParser.Helpers.AstHelpers
  import EarmarkParser.Ast.Renderer.FootnoteListRenderer
  import EarmarkParser.Ast.Renderer.HtmlRenderer
  import EarmarkParser.Ast.Renderer.TableRenderer

  @moduledoc false

  def render(blocks, context = %Context{options: %Options{}}, loose? \\ true) do
    _render(blocks, context, loose?)
  end


  defp _render(blocks, context, loose?)
  defp _render([], context, _loose?), do: context
  defp _render([block|blocks], context, loose?) do
    # IO.inspect({block.__struct__, context.value}, label: :before)
    context1 = render_block(block, clear_value(context), loose?)
    # IO.inspect(context1.value, label: :partial)
    _render(blocks, prepend(context1, context.value), loose?)
  end

  defp render_block(block, context, loose?)
  #############
  # Paragraph #
  #############
  defp render_block(%Block.Para{lnb: lnb, lines: lines, attrs: attrs}=para, context, loose?) do
    context1 = convert(lines, lnb, context)
    value    = context1.value |> Enum.reverse
    ast      =
    if loose? do
      emit("p", value, Enum.map(attrs || %{}, &attrs_to_string_keys/1))
    else
      value
    end
    ast_ = annotate(ast, para)
    prepend(context, ast_, context1.options.messages)
  end
  ########
  # Html #
  ########
  defp render_block(%Block.Html{annotation: annotation, html: html}, context, _loose?) do
    render_html_block(html, context, annotation)
  end
  defp render_block(%Block.HtmlOneline{annotation: annotation, html: html}, context, _loose?) do
    render_html_oneline(html, context, annotation)
  end
  defp render_block(%Block.HtmlComment{lines: lines}, context, _loose?) do
    lines1 =
      lines |> Enum.map(&render_html_comment_line/1)
    prepend(context, emit(:comment, lines1, [], %{comment: true}))
  end
  #########
  # Ruler #
  #########
  defp render_block(%Block.Ruler{type: "-", attrs: attrs}, context, _loose?) do
    prepend(context, emit("hr", [], merge_attrs(attrs, %{"class" => "thin"})))
  end
  defp render_block(%Block.Ruler{type: "_", attrs: attrs}, context, _loose?) do
    prepend(context, emit("hr", [], merge_attrs( attrs, %{"class" => "medium"})))
  end
  defp render_block(%Block.Ruler{type: "*", attrs: attrs}, context, _loose?) do
    prepend(context, emit("hr", [], merge_attrs(attrs, %{"class" => "thick"})))
  end
  ###########
  # Heading #
  ###########
  defp render_block(
         %Block.Heading{lnb: lnb, level: level, content: content, attrs: attrs},
         context, _loose?
       ) do
    context1 = convert(content, lnb, clear_value(context))
    modify_value(context1,
      fn _ -> [emit("h#{level}", context1.value |> Enum.reverse, Enum.map(attrs || %{}, &attrs_to_string_keys/1))] end)
  end
  ##############
  # Blockquote #
  ##############
  defp render_block(%Block.BlockQuote{blocks: blocks, attrs: attrs}, context, _loose?) do
    context1 = render(blocks, clear_value(context))
    modify_value(context1, fn ast -> [emit("blockquote", ast, Enum.map(attrs || %{}, &attrs_to_string_keys/1))] end)
  end
  #########
  # Table #
  #########
  defp render_block(
         %Block.Table{lnb: lnb, header: header, rows: rows, alignments: aligns, attrs: attrs},
         context, _loose?
       ) do
    {rows_ast, context1} = render_rows(rows, lnb, aligns, context)

    {rows_ast1, context2} =
      if header do
        {header_ast, context3} = render_header(header, lnb, aligns, context1)
        {[header_ast|rows_ast], context3}
      else
        {rows_ast, context1}
      end

    prepend(clear_value(context2), emit("table", rows_ast1, Enum.map(attrs || %{}, &attrs_to_string_keys/1)))
  end
  ########
  # Code #
  ########
  defp render_block(
         %Block.Code{language: language, attrs: attrs} = block,
         context = %Context{options: options}, _loose?
       ) do
    classes =
      if language && language != "", do: [code_classes(language, options.code_class_prefix)], else: []

    lines = render_code(block)
    prepend(context, emit("pre", emit("code", lines, classes), Enum.map(attrs || %{}, &attrs_to_string_keys/1)))
  end
  #########
  # Lists #
  #########
  @start_rgx ~r{\A\d+}
  defp render_block(
         %Block.List{type: type, bullet: bullet, blocks: items, attrs: attrs},
         context, _loose?
       ) do
    context1 = render(items, clear_value(context))
    start_map = case bullet && Regex.run(@start_rgx, bullet) do
      nil      -> %{}
      ["1"]    -> %{}
      [start1] -> %{start: _normalize_start(start1)}
    end
    prepend(context, emit(to_string(type), context1.value, merge_attrs(attrs, start_map)), context1.options.messages)

  end

  # format a spaced list item
  defp render_block(%Block.ListItem{blocks: blocks, attrs: attrs, loose?: loose?}, context, _loose?) do
    # IO.inspect blocks
    context1 = render(blocks, clear_value(context), loose?)
    # IO.inspect(context1.value, label: :list_item_blocks)
    # See below why _fix_text_lines is a NOP right now
    # prepend(context, emit("li", _fix_text_lines(context1.value, loose?), Enum.map(attrs || %{}, &attrs_to_string_keys/1)), context1.options.messages)
    prepend(context, emit("li", context1.value, Enum.map(attrs || %{}, &attrs_to_string_keys/1)), context1.options.messages)
  end

  ########
  # Text #
  ########

  defp render_block(%Block.Text{line: line, lnb: lnb}, context, loose?) do
    context1 = convert(line, lnb, clear_value(context))
    ast =  context1.value |> Enum.reverse
    if loose? do   
      modify_value(context1, fn _ -> [emit("p", ast)] end)
    else
      modify_value(context1, fn _ -> ast end) 
    end
  end

  ##################
  # Footnote Block #
  ##################

  defp render_block(%Block.FnList{blocks: footnotes}, context, _loose?) do
    items =
      Enum.map(footnotes, fn note ->
        blocks = append_footnote_link(note)
        %Block.ListItem{attrs: %{id: ["#fn:#{note.number}"]}, type: :ol, blocks: blocks}
      end)

    prepend(context, render_footnote_list(items, context))
  end

  #######################################
  # Isolated IALs are rendered as paras #
  #######################################

  defp render_block(%Block.Ial{verbatim: verbatim}, context, _loose?) do
    prepend(context, emit("p", "{:#{verbatim}}"))
  end

  ####################
  # IDDef is ignored #
  ####################

  defp render_block(%Block.IdDef{}, context, _loose?), do: context


  defp append_footnote_link(note)
  defp append_footnote_link(note = %Block.FnDef{}) do
    [last_block | blocks] = Enum.reverse(note.blocks)

    attrs = %{title: "return to article", class: "reversefootnote", href: "#fnref:#{note.number}"}
    [%{last_block|attrs: attrs} | blocks]
      |> Enum.reverse
      |> List.flatten
  end


  # Helpers
  # -------


  # Seems to be dead code but as GFM list handling is broken maybe we have a bug
  # that does not call this correctly, anyhow AST triplets do not exits anymore
  # so this code would break if called
  # defp _fix_text_lines(ast, loose?)
  # defp _fix_text_lines(ast, false), do: Enum.map(ast, &_fix_tight_text_line/1)
  # defp _fix_text_lines(ast, true), do: Enum.map(ast, &_fix_loose_text_line/1)

  # defp _fix_loose_text_line(node)
  # defp _fix_loose_text_line({:text, _, lines}), do: emit("p", lines)
  # defp _fix_loose_text_line(node), do: node

  # defp _fix_tight_text_line(node)
  # defp _fix_tight_text_line({:text, _, lines}), do: lines
  # defp _fix_tight_text_line(node), do: node

  # INLINE CANDIDATE
  defp _normalize_start(start) do
    case String.trim_leading(start, "0") do
      "" -> "0"
      start1 -> start1
    end
  end

end

# SPDX-License-Identifier: Apache-2.0
