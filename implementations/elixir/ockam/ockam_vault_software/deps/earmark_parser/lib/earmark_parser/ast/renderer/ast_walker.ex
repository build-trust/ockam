defmodule EarmarkParser.Ast.Renderer.AstWalker do
  
  @moduledoc false

  def walk(anything, fun, ignore_map_keys \\ false), do: _walk(anything, fun, ignore_map_keys, false)

  def walk_ast(ast, fun), do: _walk_ast(ast, fun, [])


  defp _walk(ast, fun, ignore_map_keys, child_of_map)
  defp _walk([], _fun, _ignore_map_keys, _child_of_map), do: []
  defp _walk(list, fun, ignore_map_keys, _child_of_map) when is_list(list) do
    Enum.map(list, &(_walk(&1, fun, ignore_map_keys, false)))
  end
  defp _walk(map, fun, ignore_map_keys, _child_of_map) when is_map(map) do
    map
      |> Enum.into(%{}, &(_walk(&1, fun, ignore_map_keys, true)))
  end
  defp _walk(tuple, fun, ignore_map_keys, child_of_map) when is_tuple(tuple) do
    if child_of_map && ignore_map_keys do
      _walk_map_element(tuple, fun, ignore_map_keys)
    else
      tuple
      |> Tuple.to_list
      |> Enum.map(&(_walk(&1, fun, ignore_map_keys, false)))
      |> List.to_tuple
    end
  end
  defp _walk(ele, fun, _ignore_map_keys, _child_of_map), do: fun.(ele)

  defp _walk_map_element({key, value}, fun, ignore_map_keys) do 
    {key, _walk(value, fun, ignore_map_keys, false)}
  end


  defp _walk_ast(ast, fun, res)
  defp _walk_ast([], _fun, res), do: Enum.reverse(res)
  defp _walk_ast(stringy, fun, res) when is_binary(stringy), do: _walk_ast([stringy], fun, res)
  defp _walk_ast([stringy|rest], fun, res) when is_binary(stringy) do
    res1 = 
    case fun.(stringy) do
      []          -> res
      [_|_]=trans -> List.flatten([Enum.reverse(trans)|res])
      stringy1    -> [stringy1|res]
    end
    _walk_ast(rest, fun, res1)
  end
  defp _walk_ast([{tag, atts, content, meta}|rest], fun, res) do
    _walk_ast(rest, fun, [{tag, atts, _walk_ast(content, fun, []), meta}|res])
  end
  defp _walk_ast([list|rest], fun, res) when is_list(list) do
    _walk_ast(rest, fun, [_walk_ast(list, fun, [])|res])
  end
end
