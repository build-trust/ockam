defmodule ExDoc.Retriever do
  # Functions to extract documentation information from modules.
  @moduledoc false

  defmodule Error do
    @moduledoc false
    defexception [:message]
  end

  alias ExDoc.{DocAST, GroupMatcher, Refs}
  alias ExDoc.Retriever.Error

  @doc """
  Extract documentation from all modules in the specified directory or directories.
  """
  @spec docs_from_dir(Path.t() | [Path.t()], ExDoc.Config.t()) :: [ExDoc.ModuleNode.t()]
  def docs_from_dir(dir, config) when is_binary(dir) do
    files = Path.wildcard(Path.expand("*.beam", dir))
    docs_from_files(files, config)
  end

  def docs_from_dir(dirs, config) when is_list(dirs) do
    Enum.flat_map(dirs, &docs_from_dir(&1, config))
  end

  @doc """
  Extract documentation from all modules in the specified list of files
  """
  @spec docs_from_files([Path.t()], ExDoc.Config.t()) :: [ExDoc.ModuleNode.t()]
  def docs_from_files(files, config) when is_list(files) do
    files
    |> Enum.map(&filename_to_module(&1))
    |> docs_from_modules(config)
  end

  @doc """
  Extract documentation from all modules in the list `modules`
  """
  @spec docs_from_modules([atom], ExDoc.Config.t()) :: [ExDoc.ModuleNode.t()]
  def docs_from_modules(modules, config) when is_list(modules) do
    modules
    |> Enum.flat_map(&get_module(&1, config))
    |> Enum.sort_by(fn module ->
      {GroupMatcher.group_index(config.groups_for_modules, module.group), module.nested_context,
       module.nested_title, module.id}
    end)
  end

  defp filename_to_module(name) do
    name = Path.basename(name, ".beam")
    String.to_atom(name)
  end

  defp get_module(module, config) do
    with {:docs_v1, _, language, _, _, _, _} = docs_chunk <- docs_chunk(module),
         {:ok, language} <- ExDoc.Language.get(language, module),
         %{} = module_data <- language.module_data(module, docs_chunk, config),
         false <- skip_module?(module_data, config) do
      [generate_node(module, module_data, config)]
    else
      _ ->
        []
    end
  end

  defp skip_module?(module_data, config) do
    !!config.filter_prefix and not String.starts_with?(module_data.id, config.filter_prefix)
  end

  defp docs_chunk(module) do
    result = ExDoc.Utils.Code.fetch_docs(module)
    Refs.insert_from_chunk(module, result)

    case result do
      {:docs_v1, _, _, _, :hidden, _, _} ->
        false

      {:docs_v1, _, _, _, _, _, _} = docs ->
        case Code.ensure_loaded(module) do
          {:module, _} ->
            docs

          {:error, reason} ->
            IO.warn("skipping module #{inspect(module)}, reason: #{reason}", [])
            false
        end

      {:error, :chunk_not_found} ->
        false

      {:error, :module_not_found} ->
        unless Code.ensure_loaded?(module) do
          raise Error, "module #{inspect(module)} is not defined/available"
        end

      {:error, _} = error ->
        raise Error, "error accessing #{inspect(module)}: #{inspect(error)}"

      _ ->
        raise Error,
              "unknown format in Docs chunk. This likely means you are running on " <>
                "a more recent Elixir version that is not supported by ExDoc. Please update."
    end
  end

  defp generate_node(module, module_data, config) do
    source_url = config.source_url_pattern
    source_path = source_path(module, config)
    source = %{url: source_url, path: source_path}

    {doc_line, moduledoc, metadata} = get_module_docs(module_data, source_path)

    groups_for_functions =
      Enum.map(config.groups_for_functions, fn {group, filter} ->
        {Atom.to_string(group), filter}
      end) ++
        [
          {"Callbacks", & &1[:__callback__]},
          {"Functions", fn _ -> true end}
        ]

    function_groups = Enum.map(groups_for_functions, &elem(&1, 0))
    function_docs = get_docs(module_data, source, groups_for_functions)
    docs = function_docs ++ get_callbacks(module_data, source, groups_for_functions)
    types = get_types(module_data, source)

    {nested_title, nested_context} = module_data.nesting_info || {nil, nil}

    node = %ExDoc.ModuleNode{
      id: module_data.id,
      title: module_data.title,
      nested_title: nested_title,
      nested_context: nested_context,
      module: module,
      type: module_data.type,
      deprecated: metadata[:deprecated],
      function_groups: function_groups,
      docs: Enum.sort_by(docs, &sort_key(&1.name, &1.arity)),
      doc: moduledoc,
      doc_line: doc_line,
      typespecs: Enum.sort_by(types, &{&1.name, &1.arity}),
      source_path: source_path,
      source_url: source_link(source, module_data.line),
      language: module_data.language
    }

    put_in(node.group, GroupMatcher.match_module(config.groups_for_modules, node))
  end

  defp sort_key(name, arity) do
    first = name |> Atom.to_charlist() |> hd()
    {first in ?a..?z, name, arity}
  end

  defp doc_ast(format, %{"en" => doc_content}, options) do
    DocAST.parse!(doc_content, format, options)
  end

  defp doc_ast(_, _, _options) do
    nil
  end

  # Module Helpers

  defp get_module_docs(module_data, source_path) do
    {:docs_v1, anno, _, content_type, moduledoc, metadata, _} = module_data.docs
    doc_line = anno_line(anno)
    options = [file: source_path, line: doc_line + 1]
    {doc_line, doc_ast(content_type, moduledoc, options), metadata}
  end

  ## Function helpers

  defp get_docs(module_data, source, groups_for_functions) do
    {:docs_v1, _, _, _, _, _, doc_elements} = module_data.docs

    nodes =
      Enum.flat_map(doc_elements, fn doc_element ->
        case module_data.language.function_data(doc_element, module_data) do
          :skip ->
            []

          function_data ->
            [get_function(doc_element, function_data, source, module_data, groups_for_functions)]
        end
      end)

    filter_defaults(nodes)
  end

  defp get_function(doc_element, function_data, source, module_data, groups_for_functions) do
    {:docs_v1, _, _, content_type, _, _, _} = module_data.docs
    {{type, name, arity}, anno, signature, doc_content, metadata} = doc_element
    doc_line = anno_line(anno)
    annotations = annotations_from_metadata(metadata) ++ function_data.extra_annotations
    line = function_data.line || doc_line
    defaults = get_defaults(name, arity, Map.get(metadata, :defaults, 0))

    doc_ast =
      (doc_content && doc_ast(content_type, doc_content, file: source.path, line: doc_line + 1)) ||
        function_data.doc_fallback.()

    group =
      Enum.find_value(groups_for_functions, fn {group, filter} ->
        filter.(metadata) && group
      end)

    %ExDoc.FunctionNode{
      id: "#{name}/#{arity}",
      name: name,
      arity: arity,
      deprecated: metadata[:deprecated],
      doc: doc_ast,
      doc_line: doc_line,
      defaults: Enum.sort_by(defaults, fn {name, arity} -> sort_key(name, arity) end),
      signature: signature(signature),
      specs: function_data.specs,
      source_path: source.path,
      source_url: source_link(source, line),
      type: type,
      group: group,
      annotations: annotations
    }
  end

  defp get_defaults(_name, _arity, 0), do: []

  defp get_defaults(name, arity, defaults) do
    for default <- (arity - defaults)..(arity - 1), do: {name, default}
  end

  defp filter_defaults(nodes) do
    Enum.map(nodes, &filter_defaults(&1, nodes))
  end

  defp filter_defaults(node, nodes) do
    update_in(node.defaults, fn defaults ->
      Enum.reject(defaults, fn {name, arity} ->
        Enum.any?(nodes, &match?(%{name: ^name, arity: ^arity}, &1))
      end)
    end)
  end

  ## Callback helpers

  defp get_callbacks(%{type: :behaviour} = module_data, source, groups_for_functions) do
    {:docs_v1, _, _, _, _, _, docs} = module_data.docs

    for {{kind, _, _}, _, _, _, _} = doc <- docs, kind in module_data.callback_types do
      get_callback(doc, source, groups_for_functions, module_data)
    end
  end

  defp get_callbacks(_, _, _), do: []

  defp get_callback(callback, source, groups_for_functions, module_data) do
    callback_data = module_data.language.callback_data(callback, module_data)

    {:docs_v1, _, _, content_type, _, _, _} = module_data.docs
    {{kind, name, arity}, anno, _signature, doc, metadata} = callback
    doc_line = anno_line(anno)

    signature = signature(callback_data.signature)
    specs = callback_data.specs
    annotations = callback_data.extra_annotations ++ annotations_from_metadata(metadata)
    doc_ast = doc_ast(content_type, doc, file: source.path, line: doc_line + 1)

    group =
      Enum.find_value(groups_for_functions, fn {group, filter} ->
        metadata = Map.put(metadata, :__callback__, true)
        filter.(metadata) && group
      end)

    %ExDoc.FunctionNode{
      id: "c:#{name}/#{arity}",
      name: name,
      arity: arity,
      deprecated: metadata[:deprecated],
      doc: doc_ast,
      doc_line: doc_line,
      signature: signature,
      specs: specs,
      source_path: source.path,
      source_url: source_link(source, callback_data.line),
      type: kind,
      annotations: annotations,
      group: group
    }
  end

  ## Typespecs

  defp get_types(module_data, source) do
    {:docs_v1, _, _, _, _, _, docs} = module_data.docs

    for {{:type, _, _}, _, _, content, _} = doc <- docs, content != :hidden do
      get_type(doc, source, module_data)
    end
  end

  defp get_type(type_entry, source, module_data) do
    {:docs_v1, _, _, content_type, _, _, _} = module_data.docs
    {{_, name, arity}, anno, _signature, doc, metadata} = type_entry
    doc_line = anno_line(anno)
    annotations = annotations_from_metadata(metadata)

    type_data = module_data.language.type_data(type_entry, module_data)
    signature = signature(type_data.signature)
    annotations = if type_data.type == :opaque, do: ["opaque" | annotations], else: annotations
    doc_ast = doc_ast(content_type, doc, file: source.path)

    %ExDoc.TypeNode{
      id: "t:#{name}/#{arity}",
      name: name,
      arity: arity,
      type: type_data.type,
      spec: type_data.spec,
      deprecated: metadata[:deprecated],
      doc: doc_ast,
      doc_line: doc_line,
      signature: signature,
      source_path: source.path,
      source_url: source_link(source, type_data.line),
      annotations: annotations
    }
  end

  ## General helpers

  defp signature([]), do: nil
  defp signature(list) when is_list(list), do: Enum.join(list, " ")

  defp annotations_from_metadata(metadata) do
    annotations = []

    annotations =
      if since = metadata[:since] do
        ["since #{since}" | annotations]
      else
        annotations
      end

    annotations
  end

  defp anno_line(line) when is_integer(line), do: abs(line)
  defp anno_line(anno), do: anno |> :erl_anno.line() |> abs()

  defp source_link(%{path: _, url: nil}, _line), do: nil

  defp source_link(source, line) do
    source_url = Regex.replace(~r/%{path}/, source.url, source.path)
    Regex.replace(~r/%{line}/, source_url, to_string(line))
  end

  defp source_path(module, _config) do
    module.module_info(:compile)[:source]
    |> String.Chars.to_string()
    |> Path.relative_to(File.cwd!())
  end
end
