defmodule EarmarkParser.Helpers.LookaheadHelpers do

  @moduledoc false
  use EarmarkParser.Types

  import EarmarkParser.Helpers.LeexHelpers

  @doc """
  Indicates if the _numbered_line_ passed in leaves an inline code block open.

  If so returns a tuple whre the first element is the opening sequence of backticks,
  and the second the linenumber of the _numbered_line_

  Otherwise `{nil, 0}` is returned
  """
  def opens_inline_code(%{line: line, lnb: lnb}) do
    case tokenize(line, with: :string_lexer) |> has_still_opening_backtix(nil) do
      nil -> {nil, 0}
      {_, btx} -> {btx, lnb}
    end
  end

  @doc """
  returns false if and only if the line closes a pending inline code
  *without* opening a new one.
  The opening backtix are passed in as second parameter.
  If the function does not return false it returns the (new or original)
  opening backtix
  """
  # (#{},{_,_}) -> {_,_}
  def still_inline_code(%{line: line, lnb: lnb}, old = {pending, _pending_lnb}) do
    case tokenize(line, with: :string_lexer) |> has_still_opening_backtix({:old, pending}) do
      nil -> {nil, 0}
      {:new, btx} -> {btx, lnb}
      {:old, _} -> old
    end
  end

  # A tokenized line {:verabtim, text} | {:backtix, ['``+]} is analyzed for
  # if it is closed (-> nil), not closed (-> {:old, btx}) or reopened (-> {:new, btx})
  # concerning backtix
  defp has_still_opening_backtix(tokens, opened_so_far)

  # Empty, done, but take care of tangeling escape (\)
  defp has_still_opening_backtix([], :force_outside), do: nil
  defp has_still_opening_backtix([], open), do: open

  # Outside state, represented by nil
  defp has_still_opening_backtix([{:other, _} | rest], nil),
    do: has_still_opening_backtix(rest, nil)

  defp has_still_opening_backtix([{:backtix, btx} | rest], nil),
    do: has_still_opening_backtix(rest, {:new, btx})

  defp has_still_opening_backtix([{:escape, _} | rest], nil),
    do: has_still_opening_backtix(rest, :force_outside)

  # Next state forced outside, represented by :force_outside
  defp has_still_opening_backtix([_ | rest], :force_outside),
    do: has_still_opening_backtix(rest, nil)

  # Inside state, represented by { :old | :new, btx }
  defp has_still_opening_backtix([{:backtix, btx} | rest], open = {_, openedbtx}) do
    if btx == openedbtx do
      has_still_opening_backtix(rest, nil)
    else
      has_still_opening_backtix(rest, open)
    end
  end

  defp has_still_opening_backtix([_ | rest], open = {_, _}),
    do: has_still_opening_backtix(rest, open)
end

# SPDX-License-Identifier: Apache-2.0
