defmodule EarmarkParser.Ast.Inline do

  @moduledoc false

  alias EarmarkParser.Context
  alias EarmarkParser.Helpers.LinkParser
  alias EarmarkParser.Helpers.PureLinkHelpers

  import EarmarkParser.Ast.Emitter
  import EarmarkParser.Ast.Renderer.AstWalker
  import EarmarkParser.Helpers
  import EarmarkParser.Helpers.AttrParser
  import EarmarkParser.Helpers.StringHelpers, only: [behead: 2]
  import EarmarkParser.Helpers.AstHelpers
  import Context, only: [set_value: 2]

  @typep conversion_data :: {String.t, non_neg_integer(), EarmarkParser.Context.t, boolean()}
  def convert(src, lnb, context)
  def convert(list, lnb, context) when is_list(list),
    do: _convert(Enum.join(list, "\n"), lnb, context, true)
  def convert(src, lnb, context), do: _convert(src, lnb, context, true)

  defp _convert(src, current_lnb, context, use_linky?)
  defp _convert(src, _, %{options: %{parse_inline: false}} = context, _) do
    prepend(context, src)
  end
  defp _convert("", _, context, _), do: context
  defp _convert(src, current_lnb, context, use_linky?) do
    case _convert_next(src, current_lnb, context, use_linky?) do
      {src1, lnb1, context1, use_linky1?} -> _convert(src1, lnb1, context1, use_linky1?)
    end
  end

  @linky_converter_names [
    :converter_for_link_and_image,
    :converter_for_reflink,
    :converter_for_footnote,
    :converter_for_nolink
  ]

  defp all_converters do
    [
      converter_for_escape: &converter_for_escape/1,
      converter_for_autolink: &converter_for_autolink/1,
      converter_for_link_and_image: &converter_for_link_and_image/1,
      converter_for_only_image: &converter_for_only_image/1,
      converter_for_reflink: &converter_for_reflink/1,
      converter_for_footnote: &converter_for_footnote/1,
      converter_for_nolink: &converter_for_nolink/1,
      converter_for_strikethrough_gfm: &converter_for_strikethrough_gfm/1,
      converter_for_strong: &converter_for_strong/1,
      converter_for_em: &converter_for_em/1,
      converter_for_code: &converter_for_code/1,
      converter_for_br: &converter_for_br/1,
      converter_for_inline_ial: &converter_for_inline_ial/1,
      converter_for_pure_link: &converter_for_pure_link/1,
      converter_for_text: &converter_for_text/1
    ]

  end

  defp _convert_next(src, lnb, context, use_linky?) do
    converters =
      if use_linky? do
        all_converters()
      else
        all_converters() |> Keyword.drop(@linky_converter_names)
      end
    _find_and_execute_converter({src, lnb, context, use_linky?}, converters)
  end

  @spec _find_and_execute_converter( conversion_data(), list ) :: conversion_data()
  defp _find_and_execute_converter({src, lnb, context, use_linky?}, converters) do
    converters
    |> Enum.find_value( fn {_converter_name, converter} -> converter.({src, lnb, context, use_linky?}) end)
  end

  ######################
  #
  #  Converters
  #
  ######################
  @escape_rule ~r{^\\([\\`*\{\}\[\]()\#+\-.!_>])}
  defp converter_for_escape({src, lnb, context, use_linky?}) do
    if match = Regex.run(@escape_rule, src) do
      [match, escaped] = match
      {behead(src, match), lnb, prepend(context, escaped), use_linky?}
    end
  end

  @autolink_rgx ~r{^<([^ >]+(@|:\/)[^ >]+)>}
  defp converter_for_autolink({src, lnb, context, use_linky?}) do
    if match = Regex.run(@autolink_rgx, src) do
      [match, link, protocol] = match
      {href, text} = convert_autolink(link, protocol)
      out = render_link(href, text)
      {behead(src, match), lnb, prepend(context, out), use_linky?}
    end
  end

  defp converter_for_pure_link({src, lnb, context, use_linky?}) do
    if context.options.pure_links do
      case PureLinkHelpers.convert_pure_link(src) do
        {ast, length} -> {behead(src, length), lnb, prepend(context, ast), use_linky?}
        _             -> nil
      end
    end
  end

  defp converter_for_link_and_image({src, lnb, context, use_linky?}) do
    match = LinkParser.parse_link(src, lnb)
    if match do
      {match1, text, href, title, link_or_img} = match
      out =
        case link_or_img do
          :link     -> output_link(context, text, href, title, lnb)
          :wikilink -> maybe_output_wikilink(context, text, href, title, lnb)
          :image    -> render_image(text, href, title)
        end

      if out do
        {behead(src, match1), lnb, prepend(context, out), use_linky?}
      end
    end
  end

  defp converter_for_only_image({src, lnb, context, use_linky?}) do
    case LinkParser.parse_link(src, lnb) do
      {match1, text, href, title, :image} ->
        out = render_image(text, href, title)
        {behead(src, match1), lnb, prepend(context, out), use_linky?}
      _ -> nil
    end
  end

  @link_text ~S{(?:\[[^]]*\]|[^][]|\])*}
  @reflink ~r{^!?\[(#{@link_text})\]\s*\[([^]]*)\]}x
  defp converter_for_reflink({src, lnb, context, use_linky?}) do
    if match = Regex.run(@reflink, src) do
      {match_, alt_text, id} =
        case match do
          [match__, id, ""] -> {match__, id, id}
          [match__, alt_text, id] -> {match__, alt_text, id}
        end

      case reference_link(context, match_, alt_text, id, lnb) do
        {:ok, out} -> {behead(src, match_), lnb, prepend(context, out), use_linky?}
        _ -> nil
      end
    end
  end

  defp converter_for_footnote({src, lnb, context, use_linky?}) do
    case Regex.run(context.rules.footnote, src) do
      [match, id] ->
        case footnote_link(context, match, id) do
          {:ok, out} -> {behead(src, match), lnb, prepend(context, out), use_linky?}
          _ -> nil
        end

      _ ->
        nil
    end
  end

  @nolink ~r{^!?\[((?:\[[^]]*\]|[^][])*)\]}
  defp converter_for_nolink({src, lnb, context, use_linky?}) do
    case Regex.run(@nolink, src) do
      [match, id] ->
        case reference_link(context, match, id, id, lnb) do
          {:ok, out} -> {behead(src, match), lnb, prepend(context, out), use_linky?}
          _ -> nil
        end

      _ ->
        nil
    end
  end

  ################################
  # Simple Tags: em, strong, del #
  ################################
  @strikethrough_rgx ~r{\A~~(?=\S)([\s\S]*?\S)~~}
  defp converter_for_strikethrough_gfm({src, _, _, _}=conv_tuple) do
    if match = Regex.run(@strikethrough_rgx, src) do
      _converter_for_simple_tag(conv_tuple, match, "del")
    end
  end
  @strong_rgx ~r{\A__([\s\S]+?)__(?!_)|^\*\*([\s\S]+?)\*\*(?!\*)}
  defp converter_for_strong({src, _, _, _}=conv_tuple) do
    if match = Regex.run(@strong_rgx, src) do
      _converter_for_simple_tag(conv_tuple, match, "strong")
    end
  end
  @emphasis_rgx ~r{\A\b_((?:__|[\s\S])+?)_\b|^\*((?:\*\*|[\s\S])+?)\*(?!\*)}
  defp converter_for_em({src, _, _, _}=conv_tuple) do
    if match = Regex.run(@emphasis_rgx, src) do
      _converter_for_simple_tag(conv_tuple, match, "em")
    end
  end

  @squash_ws ~r{\s+}
  @code ~r{^
    (`+)		# $1 = Opening run of `
    (.+?)		# $2 = The code block
    (?<!`)
    \1			# Matching closer
    (?!`)
    }xs
  defp converter_for_code({src, lnb, context, use_linky?}) do
    if match = Regex.run(@code, src) do
      [match, _, content] = match
      # Commonmark
      content1 = content
      |> String.trim()
      |> String.replace(@squash_ws, " ")

      out = codespan(content1)
      {behead(src, match), lnb, prepend(context, out), use_linky?}
    end
  end

  @inline_ial ~r<^\s*\{:\s*(.*?)\s*}>
  defp converter_for_inline_ial(conv_data)
  defp converter_for_inline_ial(
         {src, lnb, context, use_linky?}
       ) do
    if match = Regex.run(@inline_ial, src) do
      [match, ial] = match
      {context1, ial_attrs} = parse_attrs(context, ial, lnb)
      new_tags = augment_tag_with_ial(context.value, ial_attrs)
      {behead(src, match), lnb, set_value(context1, new_tags), use_linky?} # |> IO.inspect
    end
  end
  defp converter_for_inline_ial(_conv_data), do: nil

  defp converter_for_br({src, lnb, context, use_linky?}) do
    if match = Regex.run(context.rules.br, src, return: :index) do
      [{0, match_len}] = match
      {behead(src, match_len), lnb, prepend(context, emit("br")), use_linky?}
    end
  end

  @line_ending ~r{\r\n?|\n}
  @spec converter_for_text( conversion_data() ) :: conversion_data()
  defp converter_for_text({src, lnb, context, _}) do
    matched =
      case Regex.run(context.rules.text, src) do
        [match] -> match
      end

    line_count = matched |> String.split(@line_ending) |> Enum.count

    ast = hard_line_breaks(matched, context.options.gfm)
    ast = walk_ast(ast, &gruber_line_breaks/1)
    {behead(src, matched), lnb + line_count - 1, prepend(context, ast), true}
  end

  ######################
  #
  #  Helpers
  #
  ######################
  defp _converter_for_simple_tag({src, lnb, context, use_linky?}, match, for_tag) do
    {match1, content} =
      case match do
        [m, _, c] -> {m, c}
        [m, c] -> {m, c}
      end

    context1 = _convert(content, lnb, set_value(context, []), use_linky?)

    {behead(src, match1), lnb, prepend(context, emit(for_tag, context1.value|>Enum.reverse)), use_linky?}
  end


  defp convert_autolink(link, separator)
  defp convert_autolink(link, _separator = "@") do
    link = if String.at(link, 6) == ":", do: behead(link, 7), else: link
    text = link
    href = "mailto:" <> text
    {href, text}
  end
  defp convert_autolink(link, _separator) do
    {link, link}
  end

  @gruber_line_break Regex.compile!(" {2,}(?>\n)", "m")
  defp gruber_line_breaks(text) do
    text
    |> String.split(@gruber_line_break)
    |> Enum.intersperse(emit("br"))
    |> _remove_leading_empty()
  end

  @gfm_hard_line_break ~r{\\\n}
  defp hard_line_breaks(text, gfm)
  defp hard_line_breaks(text, false), do: text
  defp hard_line_breaks(text, nil), do: text
  defp hard_line_breaks(text, _) do
    text
    |> String.split(@gfm_hard_line_break)
    |> Enum.intersperse(emit("br"))
    |> _remove_leading_empty()
  end


  defp output_image_or_link(context, link_or_image, text, href, title, lnb)
  defp output_image_or_link(_context, "!" <> _, text, href, title, _lnb) do
    render_image(text, href, title)
  end
  defp output_image_or_link(context, _, text, href, title, lnb) do
    output_link(context, text, href, title, lnb)
  end

  defp output_link(context, text, href, title, lnb) do
    context1 = %{context | options: %{context.options | pure_links: false}}

    context2 = _convert(text, lnb, set_value(context1, []), false)
    if title do
      emit("a", Enum.reverse(context2.value), href: href, title: title)
    else
      emit("a", Enum.reverse(context2.value), href: href)
    end
  end

  defp maybe_output_wikilink(context, text, href, title, lnb) do
    if context.options.wikilinks do
      {tag, attrs, content, meta} = output_link(context, text, href, title, lnb)
      {tag, attrs, content, Map.put(meta, :wikilink, true)}
    end
  end

  defp reference_link(context, match, alt_text, id, lnb) do
    id = id |> replace(~r{\s+}, " ") |> String.downcase()

    case Map.fetch(context.links, id) do
      {:ok, link} ->
        {:ok, output_image_or_link(context, match, alt_text, link.url, link.title, lnb)}

      _ ->
        nil
    end
  end

  defp footnote_link(context, _match, id) do
    case Map.fetch(context.footnotes, id) do
      {:ok, %{number: number}} ->
        {:ok, render_footnote_link("fn:#{number}", "fnref:#{number}", number)}
      _ ->
        nil
    end
  end

  defp prepend(%Context{}=context, prep) do
    _prepend(context, prep)
  end

  defp _prepend(context, value)
  defp _prepend(context, [bin|rest]) when is_binary(bin) do
    _prepend(_prepend(context, bin), rest)
  end
  defp _prepend(%Context{value: [str|rest]}=context, prep) when is_binary(str) and is_binary(prep) do
    %{context | value: [str <> prep|rest]}
  end
  defp _prepend(%Context{value: value}=context, prep) when is_list(prep) do
    %{context | value: Enum.reverse(prep) ++ value}
  end
  defp _prepend(%Context{value: value}=context, prep) do
    %{context | value: [prep | value]}
  end

  defp _remove_leading_empty(list)
  defp _remove_leading_empty([""|rest]), do: rest
  defp _remove_leading_empty(list), do: list

end

# SPDX-License-Identifier: Apache-2.0
