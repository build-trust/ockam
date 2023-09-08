defmodule EarmarkParser.Helpers.HtmlHelpers do

  @moduledoc false

  import EarmarkParser.Helpers.AttrParser

  @simple_tag ~r{^<(.*?)\s*>}

  @doc false

  def augment_tag_with_ial(context, tag, ial, lnb) do
    case Regex.run( @simple_tag, tag) do
      nil ->
        nil
      ["<code class=\"inline\">", "code class=\"inline\""] ->
        tag = String.replace(tag, ~s{ class="inline"}, "")
        add_attrs(context, tag, ial, [{"class", ["inline"]}], lnb)
      _   ->
        add_attrs(context, tag, ial, [], lnb)
    end

  end


  ##############################################
  # add attributes to the outer tag in a block #
  ##############################################



  @doc false
  def add_attrs(context, text, attrs_as_string_or_map, default_attrs, lnb )
  def add_attrs(context, text, nil, [], _lnb), do: {context, text}
  def add_attrs(context, text, nil, default, lnb), do: add_attrs(context, text, %{}, default, lnb)
  def add_attrs(context, text, attrs, default, lnb) when is_binary(attrs) do
    {context1, attrs} = parse_attrs( context, attrs, lnb )
    add_attrs(context1, text, attrs, default, lnb)
  end
  def add_attrs(context, text, attrs, default, _lnb) do
    {context,
      default
      |> Map.new()
      |> Map.merge(attrs, fn _k, v1, v2 -> v1 ++ v2 end)
      |> attrs_to_string()
      |> add_to(text)}
  end

  defp attrs_to_string(attrs) do
    (for { name, value } <- attrs, do: ~s/#{name}="#{Enum.join(value, " ")}"/)
                                                  |> Enum.join(" ")
  end

  defp add_to(attrs, text) do
    attrs = if attrs == "", do: "", else: " #{attrs}"
    String.replace(text, ~r{\s?/?>}, "#{attrs}\\0", global: false)
  end

end

# SPDX-License-Identifier: Apache-2.0
