defmodule EarmarkParser.Helpers.LineHelpers do

  @moduledoc false

  alias EarmarkParser.Line

  def blank?(%Line.Blank{}),   do: true
  def blank?(_),               do: false

  # Gruber's tests have
  #
  #   para text...
  #   * and more para text
  #
  # So list markers inside paragraphs are ignored. But he also has
  #
  #   *   line
  #       * line
  #
  # And expects it to be a nested list. These seem to be in conflict
  #
  # I think the second is a better interpretation, so I commented
  # out the 2nd match below.

  def text?(%Line.Text{}),      do: true
  def text?(%Line.TableLine{}), do: true
#  def text?(%Line.ListItem{}), do: true
  def text?(_),                 do: false

  def blockquote_or_text?(%Line.BlockQuote{}), do: true
  def blockquote_or_text?(struct),             do: text?(struct)

  def indent_or_blank?(%Line.Indent{}), do: true
  def indent_or_blank?(line),           do: blank?(line)

  def blank_line_in?([]),                    do: false
  def blank_line_in?([ %Line.Blank{} | _ ]), do: true
  def blank_line_in?([ _ | rest ]),          do: blank_line_in?(rest)

end

# SPDX-License-Identifier: Apache-2.0
