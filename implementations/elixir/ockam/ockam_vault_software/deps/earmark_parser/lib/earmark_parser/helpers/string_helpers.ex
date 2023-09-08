defmodule EarmarkParser.Helpers.StringHelpers do

  @moduledoc false

  @doc """
  Remove the leading part of a string
  """
  def behead(str, ignore) when is_integer(ignore) do
    String.slice(str, ignore..-1)
  end

  def behead(str, leading_string) do
    behead(str, String.length(leading_string))
  end

  @doc """
  Remove leading spaces up to size
  """
  def behead_indent(str, size) do
    String.replace(str, ~r<\A\s{0,#{size}}>, "")
  end

  @doc """
    Returns a tuple with the prefix and the beheaded string

        iex> behead_tuple("prefixpostfix", "prefix")
        {"prefix", "postfix"}
  """
  def behead_tuple(str, lead) do
    {lead, behead(str, lead)}
  end

  def betail(str, length)
  def betail(str, length) do
    str
    |> String.slice(0, max(0,String.length(str) - length))
  end
end

# SPDX-License-Identifier: Apache-2.0
