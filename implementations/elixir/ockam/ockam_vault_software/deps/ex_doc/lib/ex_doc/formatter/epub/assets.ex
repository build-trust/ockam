defmodule ExDoc.Formatter.EPUB.Assets do
  @moduledoc false

  defmacrop embed_pattern(pattern) do
    ["formatters/epub", pattern]
    |> Path.join()
    |> Path.wildcard()
    |> Enum.map(fn path ->
      Module.put_attribute(__CALLER__.module, :external_resource, path)
      {Path.basename(path), File.read!(path)}
    end)
  end

  def dist(proglang), do: dist_js() ++ dist_css(proglang)

  defp dist_js(), do: embed_pattern("dist/*.js")
  defp dist_css(:elixir), do: embed_pattern("dist/elixir-*.css")
  defp dist_css(:erlang), do: embed_pattern("dist/erlang-*.css")

  def metainfo, do: embed_pattern("metainfo/*")
end
