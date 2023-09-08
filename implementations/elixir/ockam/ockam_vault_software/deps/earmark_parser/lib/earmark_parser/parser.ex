defmodule EarmarkParser.Parser do

  @moduledoc false
  alias EarmarkParser.Block
  alias EarmarkParser.Line
  alias EarmarkParser.LineScanner
  alias EarmarkParser.Options

  import EarmarkParser.Helpers.LookaheadHelpers, only: [opens_inline_code: 1, still_inline_code: 2]
  import EarmarkParser.Helpers.LineHelpers
  import EarmarkParser.Helpers.AttrParser
  import EarmarkParser.Helpers.ReparseHelpers
  import EarmarkParser.Message, only: [add_message: 2, add_messages: 2]
  import EarmarkParser.Parser.ListParser, only: [parse_list: 3]

  @doc """
  Given a markdown document (as either a list of lines or
  a string containing newlines), return a parse tree and
  the context necessary to render the tree.

  The options are a `%EarmarkParser.Options{}` structure. See `as_html!`
  for more details.
  """
  def parse_markdown(lines, options \\ %Options{})
  def parse_markdown(lines, options = %Options{}) when is_list(lines) do
    {blocks, links, options1} = parse(lines, options, false)

    context =
      %EarmarkParser.Context{options: options1, links: links}
      |> EarmarkParser.Context.update_context()

    if options.footnotes do
      {blocks, footnotes, options1} = handle_footnotes(blocks, context.options)
      context = put_in(context.footnotes, footnotes)
      context = put_in(context.options, options1)
      {blocks, context}
    else
      {blocks, context}
    end
  end
  def parse_markdown(lines, options) when is_binary(lines) do
    lines
    |> String.split(~r{\r\n?|\n})
    |> parse_markdown(options)
  end

  def parse(text_lines), do: parse(text_lines, %Options{}, false)
  def parse(text_lines, options = %Options{}, recursive) do
    ["" | text_lines ++ [""]]
    |> LineScanner.scan_lines(options, recursive)
    # |> IO.inspect(label: :lines)
    |> parse_lines(options, recursive)
  end

  @doc false
  # Given a list of `Line.xxx` structs, group them into related blocks.
  # Then extract any id definitions, and build a map from them. Not
  # for external consumption.

  def parse_lines(lines, options, recursive) do
    {blocks, options} = lines |> remove_trailing_blank_lines() |> lines_to_blocks(options, recursive)
    links  = links_from_blocks(blocks)
    {blocks, links, options}
  end

  defp lines_to_blocks(lines, options, recursive) do
    with {blocks, options1} <- lines |> _parse([], options, recursive) do
      # { blocks |> IO.inspect(label: :blocks)|> assign_attributes_to_blocks([]) |> consolidate_list_items([]) , options1 }
      { blocks |> assign_attributes_to_blocks([]) |> consolidate_list_items([]) , options1 }
    end
  end


  defp _parse([], result, options, _recursive), do: {result, options}

  ###################
  # setext headings #
  ###################

  defp _parse([ %Line.Blank{},
                %Line.Text{content: heading, lnb: lnb},
                %Line.SetextUnderlineHeading{annotation: annotation, level: level}
             |
                rest
             ], result, options, recursive) do

    _parse(rest, [%Block.Heading{annotation: annotation, content: heading, level: level, lnb: lnb}|result], options, recursive)
  end

  defp _parse([  %Line.Blank{},
                %Line.Text{content: heading, lnb: lnb},
                %Line.Ruler{type: "-"}
             |
                rest
             ], result, options, recursive) do

    _parse(rest, [%Block.Heading{content: heading, level: 2, lnb: lnb} | result], options, recursive)
  end

  #################
  # Other heading #
  #################

  defp _parse([ %Line.Heading{content: content, ial: ial, level: level, lnb: lnb} | rest ], result, options, recursive) do

    {options1, result1} = prepend_ial(
      options, ial, lnb, [%Block.Heading{content: content, level: level, lnb: lnb} | result])
    _parse(rest, result1, options1, recursive)
  end

  #########
  # Ruler #
  #########

  defp _parse([ %Line.Ruler{type: type, lnb: lnb} | rest], result, options, recursive) do
    _parse(rest, [%Block.Ruler{type: type, lnb: lnb} | result], options, recursive)
  end

  ###############
  # Block Quote #
  ###############

  defp _parse( lines = [ %Line.BlockQuote{lnb: lnb} | _ ], result, options, recursive) do
    {quote_lines, rest} = Enum.split_while(lines, &blockquote_or_text?/1)
    lines = for line <- quote_lines, do: line.content
    {blocks, _, options1} = parse(lines, %{options | line: lnb}, true)
    _parse(rest, [%Block.BlockQuote{blocks: blocks, lnb: lnb}|result], options1, recursive)
  end

  #########
  # Table #
  #########

  defp _parse( lines = [ %Line.TableLine{columns: cols1, lnb: lnb1, needs_header: false},
                        %Line.TableLine{columns: cols2}
                      | _rest
                      ], result, options, recursive)
  when length(cols1) == length(cols2)
  do
    columns = length(cols1)
    { table, rest } = read_table(lines, columns, Block.Table.new_for_columns(columns))
    table1          = %{table | lnb: lnb1}
    _parse(rest, [ table1 | result ], options, recursive)
  end

  defp _parse( lines = [ %Line.TableLine{columns: cols1, lnb: lnb1, needs_header: true},
                        %Line.TableLine{columns: cols2, is_header: true}
                      | _rest
                      ], result, options, recursive)
  when length(cols1) == length(cols2)
  do
    columns = length(cols1)
    { table, rest } = read_table(lines, columns, Block.Table.new_for_columns(columns))
    table1          = %{table | lnb: lnb1}
    _parse(rest, [ table1 | result ], options, recursive)
  end
  #############
  # Paragraph #
  #############

  defp _parse( lines = [ %Line.TableLine{lnb: lnb} | _ ], result, options, recursive) do
    {para_lines, rest} = Enum.split_while(lines, &text?/1)
    line_text = (for line <- para_lines, do: line.line)
    _parse(rest, [ %Block.Para{lines: line_text, lnb: lnb + 1} | result ], options, recursive)
  end

  defp _parse( lines = [ %Line.Text{lnb: lnb} | _ ], result, options, recursive)
  do
    {reversed_para_lines, rest, pending, annotation} = consolidate_para(lines)

    options1 =
      case pending do
        {nil, _} -> options
        {pending, lnb1} ->
          add_message(options, {:warning, lnb1, "Closing unclosed backquotes #{pending} at end of input"})
      end

    line_text = (for line <- (reversed_para_lines |> Enum.reverse), do: line.line)
    if recursive == :list do
        _parse(rest, [ %Block.Text{line: line_text, lnb: lnb} | result ], options1, recursive)
    else
        _parse(rest, [ %Block.Para{annotation: annotation, lines: line_text, lnb: lnb} | result ], options1, recursive)
    end
  end

  defp _parse([%Line.SetextUnderlineHeading{line: line, lnb: lnb, level: 2} | rest], result, options, recursive) do
    _parse([%Line.Text{line: line, lnb: lnb} | rest], result, options, recursive)
  end

  #########
  # Lists #
  #########
  # We handle lists in two passes. In the first, we build list items,
  # in the second we combine adjacent items into lists. This is pass one

  defp _parse( [ %Line.ListItem{}|_ ]=input, result, options, recursive) do

    {with_prepended_lists, rest, options1} = parse_list(input, result, options)
    _parse([%Line.Blank{lnb: 0} | rest], with_prepended_lists, options1, recursive)

  end

  #################
  # Indented code #
  #################

  defp _parse( list = [%Line.Indent{lnb: lnb} | _], result, options, recursive) do
    {code_lines, rest} = Enum.split_while(list, &indent_or_blank?/1)
    code_lines = remove_trailing_blank_lines(code_lines)
    code = (for line <- code_lines, do: properly_indent(line, 1))
    _parse(rest, [ %Block.Code{lines: code, lnb: lnb} | result ], options, recursive)
  end

  ###############
  # Fenced code #
  ###############

  defp _parse([%Line.Fence{delimiter: delimiter, language: language, lnb: lnb} | rest], result, options, recursive) do
    {code_lines, rest} = Enum.split_while(rest, fn (line) ->
      !match?(%Line.Fence{delimiter: ^delimiter, language: _}, line)
    end)
    rest = if length(rest) == 0, do: rest, else: tl(rest)
    code = (for line <- code_lines, do: line.line)
    _parse(rest, [ %Block.Code{lines: code, language: language, lnb: lnb} | result ], options, recursive)
  end

  ##############
  # HTML block #
  ##############
  defp _parse([ opener = %Line.HtmlOpenTag{annotation: annotation, tag: tag, lnb: lnb} | rest], result, options, recursive) do
    {html_lines, rest1, unclosed, annotation} = _html_match_to_closing(opener, rest, annotation) #|> IO.inspect(label: :found) 
    options1 = add_messages(options,
                            unclosed
                            |> Enum.map(fn %{lnb: lnb1, tag: tag} -> {:warning, lnb1, "Failed to find closing <#{tag}>"} end))
    html = Enum.reverse(html_lines)
    _parse(rest1, [ %Block.Html{tag: tag, html: html, lnb: lnb, annotation: annotation} | result ], options1, recursive)
  end

  ####################
  # HTML on one line #
  ####################

  defp _parse([ %Line.HtmlOneLine{annotation: annotation, line: line, lnb: lnb} | rest], result, options, recursive) do
    _parse(rest, [ %Block.HtmlOneline{annotation: annotation, html: [ line ], lnb: lnb} | result ], options, recursive)
  end

  ################
  # HTML Comment #
  ################

  defp _parse([ line = %Line.HtmlComment{complete: true, lnb: lnb} | rest], result, options, recursive) do
    _parse(rest, [ %Block.HtmlComment{lines: [ line.line ], lnb: lnb} | result ], options, recursive)
  end

  defp _parse(lines = [ %Line.HtmlComment{complete: false, lnb: lnb} | _], result, options, recursive) do
    {html_lines, rest} = Enum.split_while(lines, fn (line) ->
      !(line.line =~ ~r/-->/)
    end)
    {html_lines, rest} = if length(rest) == 0 do
      {html_lines, rest}
    else
      {html_lines ++ [ hd(rest) ], tl(rest)}
    end
    html = (for line <- html_lines, do: line.line)
    _parse(rest, [ %Block.HtmlComment{lines: html, lnb: lnb} | result ], options, recursive)
  end

  #################
  # ID definition #
  #################

  # the title may be on the line following the iddef
  defp _parse( [ defn = %Line.IdDef{title: title, lnb: lnb}, maybe_title | rest ], result, options, recursive)
  when title == nil
  do
    title = case maybe_title do
      %Line.Text{content: content}   ->  LineScanner.matches_id_title(content)
      _                              ->  nil
    end

    if title do
      _parse(rest, [ %Block.IdDef{id: defn.id, url: defn.url, title: title, lnb: lnb} | result], options, recursive)
    else
      _parse([maybe_title | rest], [ %Block.IdDef{id: defn.id, url: defn.url, lnb: lnb} | result], options, recursive)
    end
  end

  # or not
  defp _parse( [ defn = %Line.IdDef{lnb: lnb} | rest ], result, options, recursive) do
    _parse(rest, [ %Block.IdDef{id: defn.id, url: defn.url, title: defn.title, lnb: lnb} | result], options, recursive)
  end

  #######################
  # Footnote Definition #
  #######################

  defp _parse( [ defn = %Line.FnDef{id: _id, lnb: lnb} | rest ], result , options, recursive) do
    {para_lines, rest} = Enum.split_while(rest, &text?/1)
    first_line = %Line.Text{line: defn.content, lnb: lnb}
    {para, options1} = _parse([ first_line | para_lines ], [], options, recursive)
    {indent_lines, rest} = Enum.split_while(rest, &indent_or_blank?/1)
    {blocks, _, options2} = remove_trailing_blank_lines(indent_lines)
                |> Enum.map(&(properly_indent(&1, 1)))
                |> parse(%{options1 | line: lnb + 1}, true)
    blocks = Enum.concat(para, blocks)
    _parse( rest, [ %Block.FnDef{id: defn.id, blocks: blocks , lnb: lnb} | result ], options2, recursive)
  end

  ####################
  # IAL (attributes) #
  ####################

  defp _parse( [ %Line.Ial{attrs: attrs, lnb: lnb, verbatim: verbatim} | rest ], result, options, recursive) do
    {options1, attributes} = parse_attrs( options, attrs, lnb )
    _parse(rest, [ %Block.Ial{attrs: attributes, content: attrs, lnb: lnb, verbatim: verbatim} | result ], options1, recursive)
  end

  ###############
  # Blank Lines #
  ###############
  # We've reached the point where empty lines are no longer significant

  defp _parse( [ %Line.Blank{} | rest ], result, options, recursive) do
    _parse(rest, result, options, recursive)
  end

  ##############################################################
  # Anything else... we warn, then treat it as if it were text #
  ##############################################################

  defp _parse( [ anything = %{lnb: lnb} | rest ], result, options, recursive) do
    _parse( [ %Line.Text{content: anything.line, lnb: lnb} | rest], result,
      add_message(options, {:warning, anything.lnb, "Unexpected line #{anything.line}"}), recursive)
  end

  #######################################################
  # Assign attributes that follow a block to that block #
  #######################################################

  defp assign_attributes_to_blocks([], result) do
    Enum.reverse(result)
  end
  defp assign_attributes_to_blocks([ %Block.Ial{attrs: attrs}, block | rest], result) do
    assign_attributes_to_blocks(rest, [ %{block | attrs: attrs} | result ])
  end
  defp assign_attributes_to_blocks([ block | rest], result) do
    assign_attributes_to_blocks(rest, [ block | result ])
  end

  ############################################################
  # Consolidate multiline inline code blocks into an element #
  ############################################################
  @not_pending {nil, 0}
  # ([#{},...]) -> {[#{}],[#{}],{'nil' | binary(),number()}}
  # @spec consolidate_para( ts ) :: { ts, ts, {nil | String.t, number} }
  defp consolidate_para( lines ), do: _consolidate_para( lines, [], @not_pending, nil )

  defp _consolidate_para( [], result, pending, annotation ) do
    {result, [], pending, annotation}
  end

  defp _consolidate_para( [line | rest] = lines, result, pending, annotation ) do
    # IO.inspect({line, annotation}, label: :consolidate)
    case _inline_or_text?( line, pending ) do
      %{pending: still_pending, continue: true} -> _consolidate_para( rest, [line | result], still_pending, annotation || line.annotation )
      _                                         -> {result, lines, @not_pending, annotation}
    end

  end

  ##################################################
  # Consolidate one or more list items into a list #
  ##################################################

  defp consolidate_list_items([], result) do
    result |> Enum.map(&compute_list_spacing/1)  # no need to reverse
  end
  # We have a list, and the next element is an item of the same type
  defp consolidate_list_items(
    [list = %Block.List{type: type, blocks: items},
     item = %Block.ListItem{type: type} | rest], result)
  do
    start = extract_start(item)
    items = [ item | items ]   # original list is reversed
    consolidate_list_items([ %{ list | blocks: items, start: start } | rest ], result)
  end
  # We have an item, but no open list
  defp consolidate_list_items([ item = %Block.ListItem{type: type} | rest], result) do
    start = extract_start(item)
    consolidate_list_items([ %Block.List{ type: type, blocks: [ item ], start: start} | rest ], result)
  end
  # Nothing to see here, move on
  defp consolidate_list_items([ head | rest ], result) do
    consolidate_list_items(rest, [ head | result ])
  end

  defp compute_list_spacing( list = %Block.List{blocks: items} ) do
    with spaced = any_spaced_items?(items),
         unified_items = Enum.map(items, &(%{&1 | spaced: spaced}))
    do
      %{list | blocks: unified_items}
    end
  end
  defp compute_list_spacing( anything_else ), do: anything_else # nop

  defp any_spaced_items?([]), do: false
  defp any_spaced_items?([%{spaced: true}|_]), do: true
  defp any_spaced_items?([_|tail]), do: any_spaced_items?(tail)


  ##################################################
  # Read in a table (consecutive TableLines with
  # the same number of columns)

  defp read_table(lines, col_count, into_table)
  defp read_table([ %Line.TableLine{columns: cols} | rest ],
                    col_count,
                    table = %Block.Table{})
  when length(cols) == col_count
  do
    read_table(rest, col_count, update_in(table.rows, &[ cols | &1 ]))
  end
  defp read_table( rest, col_count, %Block.Table{rows: rows}) do
    rows  = Enum.reverse(rows)
    table = Block.Table.new_for_columns(col_count)
    table = case look_for_alignments(rows) do
      nil    -> %Block.Table{table | rows: rows }
      aligns -> %Block.Table{table | alignments: aligns,
                               header:     hd(rows),
                               rows:       tl(tl(rows)) }
    end
    { table , [%Line.Blank{lnb: 0} |rest] }
  end


  defp look_for_alignments([ _first, second | _rest ]) do
    if Enum.all?(second, fn row -> row =~ ~r{^:?-+:?$} end) do
      second
      |> Enum.map(fn row -> Regex.replace(~r/-+/, row, "-") end)
      |> Enum.map(fn row -> case row do
           ":-:" -> :center
           ":-"  -> :left
           "-"   -> :left
           "-:"  -> :right
         end
      end)
    else
      nil
    end
  end


  #####################################################
  # Traverse the block list and build a list of links #
  #####################################################

  defp links_from_blocks(blocks) do
    visit(blocks, Map.new, &link_extractor/2)
  end

  defp link_extractor(item = %Block.IdDef{id: id}, result) do
    Map.put(result, String.downcase(id), item)
  end

  defp link_extractor(_, result), do: result


  ##################################
  # Visitor pattern for each block #
  ##################################

  defp visit([], result, _func), do: result

  # Structural node BlockQuote -> descend
  defp visit([ item = %Block.BlockQuote{blocks: blocks} | rest], result, func) do
    result = func.(item, result)
    result = visit(blocks, result, func)
    visit(rest, result, func)
  end

  # Structural node List -> descend
  defp visit([ item = %Block.List{blocks: blocks} | rest], result, func) do
    result = func.(item, result)
    result = visit(blocks, result, func)
    visit(rest, result, func)
  end

  # Structural node ListItem -> descend
  defp visit([ item = %Block.ListItem{blocks: blocks} | rest], result, func) do
    result = func.(item, result)
    result = visit(blocks, result, func)
    visit(rest, result, func)
  end

  # Leaf, leaf it alone
  defp visit([ item | rest], result, func) do
    result = func.(item, result)
    visit(rest, result, func)
  end

  ###################################################################
  # Consume HTML, taking care of nesting. Assumes one tag per line. #
  ###################################################################

  defp _html_match_to_closing(opener, rest, annotation), do: _find_closing_tags([opener], rest, [String.trim_leading(opener.line)], annotation)

  defp _find_closing_tags(needed, input, html_lines, annotation)
  # No more open tags, happy case
  defp _find_closing_tags([], rest, html_lines, annotation), do: {html_lines, rest, [], annotation}
  # run out of input, unhappy case
  defp _find_closing_tags(needed, [], html_lines, annotation), do: {html_lines, [], needed, annotation}
  # still more lines, still needed closing
  defp _find_closing_tags(needed = [needed_hd|needed_tl], [rest_hd|rest_tl], html_lines, annotation) do
    cond do
      _closes_tag?(rest_hd, needed_hd) -> _find_closing_tags(needed_tl, rest_tl, [String.trim_leading(rest_hd.line)|html_lines], _override_annotation(annotation, rest_hd))
      _opens_tag?(rest_hd)             -> _find_closing_tags([rest_hd|needed], rest_tl, [String.trim_leading(rest_hd.line)|html_lines], annotation)
      true                             -> _find_closing_tags(needed, rest_tl, [rest_hd.line|html_lines], annotation)
    end
  end

  ###########
  # Helpers #
  ###########

  defp _closes_tag?(%Line.HtmlCloseTag{tag: ctag}, %Line.HtmlOpenTag{tag: otag}) do
    ctag == otag
  end
  defp _closes_tag?(_, _), do: false

  defp _opens_tag?(%Line.HtmlOpenTag{}), do: true
  defp _opens_tag?(_), do: false


  defp _inline_or_text?(line, pending)
  defp _inline_or_text?(line = %Line.Text{}, @not_pending) do
    pending = opens_inline_code(line)
    %{pending: pending, continue: true}
  end
  defp _inline_or_text?(line = %Line.TableLine{}, @not_pending) do
    pending = opens_inline_code(line)
    %{pending: pending, continue: true}
  end
  defp _inline_or_text?( _line, @not_pending), do: %{pending: @not_pending, continue: false}
  defp _inline_or_text?( line, pending ) do
    pending = still_inline_code(line, pending)
    %{pending: pending, continue: true}
  end


  defp _override_annotation(annotation, line), do: annotation || line.annotation

  @start_number_rgx ~r{\A0*(\d+)[.)]}
  defp extract_start(%{bullet: bullet}) do
    case Regex.run(@start_number_rgx, bullet) do
      nil -> ""
      [_, "1"] -> ""
      [_, start] -> ~s{ start="#{start}"}
    end
  end

  defp remove_trailing_blank_lines(lines) do
    lines
    |> Enum.reverse
    |> Enum.drop_while(&blank?/1)
    |> Enum.reverse
  end
  ################################################################
  # Traverse the block list and extract the footnote definitions #
  ################################################################

  # @spec handle_footnotes( Block.ts, %EarmarkParser.Options{}, ( Block.ts,
  defp handle_footnotes(blocks, options) do
    {footnotes, blocks} = Enum.split_with(blocks, &footnote_def?/1)

    {footnotes, undefined_footnotes} =
      Options.get_mapper(options).(blocks, &find_footnote_links/1)
      |> List.flatten()
      |> get_footnote_numbers(footnotes, options)

    blocks = create_footnote_blocks(blocks, footnotes)
    footnotes = Options.get_mapper(options).(footnotes, &{&1.id, &1}) |> Enum.into(Map.new())
    options1 = add_messages(options, undefined_footnotes)
    {blocks, footnotes, options1}
  end

  defp footnote_def?(%Block.FnDef{}), do: true
  defp footnote_def?(_block), do: false

  defp find_footnote_links(%Block.Para{lines: lines, lnb: lnb}) do
    lines
    |> Enum.zip(Stream.iterate(lnb, &(&1 + 1)))
    |> Enum.flat_map(&extract_footnote_links/1)
  end

  defp find_footnote_links(%{blocks: blocks}) do
    Enum.flat_map(blocks, &find_footnote_links/1)
  end

  defp find_footnote_links(_), do: []

  defp extract_footnote_links({line, lnb}) do
    Regex.scan(~r{\[\^([^\]]+)\]}, line)
    |> Enum.map(&tl/1)
    |> Enum.zip(Stream.cycle([lnb]))
  end

  def get_footnote_numbers(refs, footnotes, options) do
    Enum.reduce(refs, {[], []}, fn {ref, lnb}, {defined, undefined} ->
      r = hd(ref)

      case Enum.find(footnotes, &(&1.id == r)) do
        note = %Block.FnDef{} ->
          number = length(defined) + options.footnote_offset
          note = %Block.FnDef{note | number: number}
          {[note | defined], undefined}

        _ ->
          {defined,
           [{:error, lnb, "footnote #{r} undefined, reference to it ignored"} | undefined]}
      end
    end)
  end

  defp create_footnote_blocks(blocks, []), do: blocks

  defp create_footnote_blocks(blocks, footnotes) do
    lnb =
      footnotes
      |> Stream.map(& &1.lnb)
      |> Enum.min()

    footnote_block = %Block.FnList{blocks: Enum.sort_by(footnotes, & &1.number), lnb: lnb}
    Enum.concat(blocks, [footnote_block])
  end

  def prepend_ial(context, maybeatts, lnb, result)
  def prepend_ial(context, nil, _lnb, result), do: {context, result}
  def prepend_ial(context, ial, lnb, result) do
    {context1, attributes} = parse_attrs(context, ial, lnb)
    {context1, [%Block.Ial{attrs: attributes, content: ial, lnb: lnb, verbatim: ial}|result]}
  end
end

# SPDX-License-Identifier: Apache-2.0
