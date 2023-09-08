defmodule ExDoc.Utils.Code do
  @moduledoc false

  # TODO: this is vendored from Elixir v1.11.0.
  #       Remove and use Code.fetch_docs/1 in the future.

  def fetch_docs(module) when is_atom(module) do
    case :code.get_object_code(module) do
      {_module, bin, beam_path} ->
        case fetch_docs_from_beam(bin) do
          {:error, :chunk_not_found} ->
            if :code.which(module) == :preloaded do
              # TODO remove duplication
              path = Path.join([:code.lib_dir(:erts), "doc", "chunks", "#{module}.chunk"])
              fetch_docs_from_chunk(path)
            else
              app_root = Path.expand(Path.join(["..", ".."]), beam_path)
              path = Path.join([app_root, "doc", "chunks", "#{module}.chunk"])
              fetch_docs_from_chunk(path)
            end

          other ->
            other
        end

      :error ->
        case :code.which(module) do
          :preloaded ->
            path = Path.join([:code.lib_dir(:erts), "doc", "chunks", "#{module}.chunk"])
            fetch_docs_from_chunk(path)

          _ ->
            {:error, :module_not_found}
        end
    end
  end

  def fetch_docs(path) when is_binary(path) do
    fetch_docs_from_beam(String.to_charlist(path))
  end

  @docs_chunk 'Docs'

  defp fetch_docs_from_beam(bin_or_path) do
    case :beam_lib.chunks(bin_or_path, [@docs_chunk]) do
      {:ok, {_module, [{@docs_chunk, bin}]}} ->
        load_docs_chunk(bin)

      {:error, :beam_lib, {:missing_chunk, _, @docs_chunk}} ->
        {:error, :chunk_not_found}

      {:error, :beam_lib, {:file_error, _, :enoent}} ->
        {:error, :module_not_found}
    end
  end

  defp fetch_docs_from_chunk(path) do
    case File.read(path) do
      {:ok, bin} ->
        load_docs_chunk(bin)

      {:error, _} ->
        {:error, :chunk_not_found}
    end
  end

  defp load_docs_chunk(bin) do
    :erlang.binary_to_term(bin)
  rescue
    _ ->
      {:error, {:invalid_chunk, bin}}
  end
end
