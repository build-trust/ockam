defmodule Makeup.Token.Utils.Hierarchy do
  @moduledoc false

  def hierarchy_to_precedence(hierarchy) do
    hierarchy
    |> Enum.map(&dependencies/1)
    |> List.flatten
    |> Enum.reverse
  end

  def node_tag({tag, _, _}), do: tag
  def node_tag({tag, _}), do: tag

  defp descendants({_, _, children}) do
    first_degree = Enum.map(children, &node_tag/1)
    higher_degree = children |> Enum.map(&descendants/1)
    (first_degree ++ higher_degree) |> List.flatten
  end
  defp descendants(_terminal), do: []

  defp dependencies({tag, _, children} = node) do
    node_dependencies = {tag, descendants(node)}
    children_dependencies = children
      |> Enum.map(&dependencies/1)
      |> List.flatten

    [node_dependencies | children_dependencies]
  end
  defp dependencies(_terminal), do: []

  def to_nested_list_of_pairs({tag, class, children}) do
    [{tag, class} | Enum.map(children, &to_nested_list_of_pairs/1)]
  end
  def to_nested_list_of_pairs({tag, class}) do
    {tag, class}
  end

  def style_to_class_map(hierarchy) do
    hierarchy
    |> Enum.map(&to_nested_list_of_pairs/1)
    |> List.flatten
    |> Enum.into(%{})
  end

end