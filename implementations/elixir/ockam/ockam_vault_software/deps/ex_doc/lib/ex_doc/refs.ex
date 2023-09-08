defmodule ExDoc.Refs do
  @moduledoc false

  # A read-through cache of documentation references.
  #
  # A given ref is always associated with a module. If we don't have a ref
  # in the cache we fetch the module's docs chunk and fill in the cache.
  #
  # If the module does not have the docs chunk, we fetch it's functions,
  # callbacks and types from other sources.

  @typep entry() :: {ref(), visibility()}

  @typep ref() ::
           {:module, module()}
           | {kind(), module(), name :: atom(), arity()}
  @typep kind() :: :function | :callback | :type
  @typep visibility() :: :hidden | :public | :undefined

  @name __MODULE__

  use GenServer

  @spec start_link(any()) :: GenServer.on_start()
  def start_link(arg) do
    GenServer.start_link(__MODULE__, arg, name: @name)
  end

  @spec init(any()) :: {:ok, nil}
  def init(_) do
    :ets.new(@name, [:named_table, :public, :set])
    {:ok, nil}
  end

  @spec clear() :: :ok
  def clear() do
    :ets.delete_all_objects(@name)
    :ok
  end

  @spec get_visibility(ref()) :: visibility()
  def get_visibility(ref) do
    case lookup(ref) do
      {:ok, visibility} ->
        visibility

      :error ->
        fetch(ref)
    end
  end

  defp lookup(ref) do
    case :ets.lookup(@name, ref) do
      [{^ref, visibility}] ->
        {:ok, visibility}

      [] ->
        :error
    end
  rescue
    _ ->
      :error
  end

  @spec insert([entry()]) :: :ok
  def insert(entries) do
    true = :ets.insert(@name, entries)
    :ok
  end

  @spec insert_from_chunk(module, tuple()) :: :ok
  def insert_from_chunk(module, result) do
    entries = fetch_entries(module, result)
    insert(entries)
    :ok
  end

  defp fetch({:module, module} = ref) do
    entries = fetch_entries(module, ExDoc.Utils.Code.fetch_docs(module))
    insert(entries)
    Map.get(Map.new(entries), ref, :undefined)
  end

  defp fetch({_kind, module, _name, _arity} = ref) do
    with module_visibility <- fetch({:module, module}),
         true <- module_visibility in [:public, :hidden],
         {:ok, visibility} <- lookup(ref) do
      visibility
    else
      _ ->
        :undefined
    end
  end

  defp fetch_entries(module, result) do
    case result do
      {:docs_v1, _, _, _, module_doc, _, docs} ->
        module_visibility = visibility(module_doc)

        for {{kind, name, arity}, _, _, doc, metadata} <- docs do
          ref_kind = to_ref_kind(kind)
          visibility = visibility(module_doc, {ref_kind, name, doc})

          for arity <- (arity - (metadata[:defaults] || 0))..arity do
            {{ref_kind, module, name, arity}, visibility}
          end
        end
        |> List.flatten()
        |> Enum.concat([
          {{:module, module}, module_visibility}
          | to_refs(types(module, [:typep]), module, :type, :hidden)
        ])

      {:error, _} ->
        if Code.ensure_loaded?(module) do
          (to_refs(exports(module), module, :function) ++
             to_refs(callbacks(module), module, :callback) ++
             to_refs(types(module, [:type, :opaque]), module, :type) ++
             to_refs(types(module, [:typep]), module, :type, :hidden))
          |> Enum.concat([{{:module, module}, :public}])
        else
          [{{:module, module}, :undefined}]
        end
    end
  end

  defguardp has_no_docs(doc) when doc == :none or doc == %{}

  defp starts_with_underscore?(name), do: hd(Atom.to_charlist(name)) == ?_

  defp visibility(:hidden),
    do: :hidden

  defp visibility(_module_doc),
    do: :public

  defp visibility(_module_doc, {kind, _name, _doc})
       when kind not in [:callback, :function, :type],
       do: raise(ArgumentError, "Unknown kind #{inspect(kind)}")

  defp visibility(:hidden, {_kind, _name, _doc}),
    do: :hidden

  defp visibility(_, {_kind, _name, :hidden}),
    do: :hidden

  defp visibility(_, {kind, name, doc}) when has_no_docs(doc) do
    cond do
      kind in [:callback, :type] ->
        :public

      kind == :function and starts_with_underscore?(name) ->
        :hidden

      kind == :function ->
        :public
    end
  end

  defp visibility(_, {_, _, _}) do
    :public
  end

  defp to_ref_kind(:macro), do: :function
  defp to_ref_kind(:macrocallback), do: :callback
  defp to_ref_kind(other), do: other

  defp exports(module) do
    if function_exported?(module, :__info__, 1) do
      module.__info__(:functions) ++ module.__info__(:macros)
    else
      module.module_info(:exports)
    end
  end

  defp callbacks(module) do
    if function_exported?(module, :behaviour_info, 1) do
      module.behaviour_info(:callbacks)
    else
      []
    end
  end

  defp types(module, kind_list) do
    case Code.Typespec.fetch_types(module) do
      {:ok, list} ->
        for {kind, {name, _, args}} <- list,
            kind in kind_list do
          {name, length(args)}
        end

      :error ->
        []
    end
  end

  defp to_refs(list, module, kind, visibility \\ :public) do
    for {name, arity} <- list do
      {{kind, module, name, arity}, visibility}
    end
  end
end
