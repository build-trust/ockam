defmodule ExDoc.Language.Erlang do
  @moduledoc false

  @behaviour ExDoc.Language

  alias ExDoc.{Autolink, Refs}

  @impl true
  # TODO: Move :hidden handling to retriever, as it is shared across all BEAM languages
  def module_data(module, docs_chunk, _config) do
    {:docs_v1, _, _, _, doc, _, _} = docs_chunk

    if doc != :hidden do
      module_data(module, docs_chunk)
    else
      :skip
    end
  end

  def module_data(module, docs_chunk) do
    # Make sure the module is loaded for future checks
    _ = Code.ensure_loaded(module)
    id = Atom.to_string(module)
    abst_code = get_abstract_code(module)
    line = find_module_line(module, abst_code)
    type = module_type(module)
    optional_callbacks = type == :behaviour && module.behaviour_info(:optional_callbacks)

    %{
      module: module,
      docs: docs_chunk,
      language: __MODULE__,
      id: id,
      title: id,
      type: type,
      line: line,
      callback_types: [:callback],
      nesting_info: nil,
      private: %{
        abst_code: abst_code,
        specs: get_specs(module),
        callbacks: get_callbacks(module),
        optional_callbacks: optional_callbacks
      }
    }
  end

  @impl true
  def function_data(entry, module_data) do
    {{kind, name, arity}, _anno, _signature, doc_content, _metadata} = entry

    # TODO: Edoc on Erlang/OTP24.1+ includes private functions in
    # the chunk, so we manually yank them out for now.
    if kind == :function and doc_content != :hidden and
         function_exported?(module_data.module, name, arity) do
      function_data(name, arity, doc_content, module_data)
    else
      :skip
    end
  end

  defp function_data(name, arity, _doc_content, module_data) do
    specs =
      case Map.fetch(module_data.private.specs, {name, arity}) do
        {:ok, specs} ->
          [{:attribute, 0, :spec, {{name, arity}, specs}}]

        :error ->
          []
      end

    %{
      doc_fallback: fn -> nil end,
      extra_annotations: [],
      line: nil,
      specs: specs
    }
  end

  @impl true
  def callback_data(entry, module_data) do
    {{_kind, name, arity}, anno, signature, _doc, _metadata} = entry

    extra_annotations =
      if {name, arity} in module_data.private.optional_callbacks, do: ["optional"], else: []

    specs =
      case Map.fetch(module_data.private.callbacks, {name, arity}) do
        {:ok, specs} ->
          [{:attribute, 0, :callback, {{name, arity}, specs}}]

        :error ->
          []
      end

    %{
      line: anno_line(anno),
      signature: signature,
      specs: specs,
      extra_annotations: extra_annotations
    }
  end

  @impl true
  def type_data(entry, module_data) do
    {{kind, name, arity}, anno, signature, _doc, _metadata} = entry

    case ExDoc.Language.Elixir.type_from_module_data(module_data, name, arity) do
      %{} = map ->
        %{
          type: map.type,
          line: map.line,
          spec: {:attribute, 0, map.type, map.spec},
          signature: signature
        }

      nil ->
        %{
          type: kind,
          line: anno_line(anno),
          spec: nil,
          signature: signature
        }
    end
  end

  @impl true
  def autolink_doc(ast, opts) do
    config = struct!(Autolink, opts)
    walk_doc(ast, config)
  end

  @impl true
  def autolink_spec(nil, _opts) do
    nil
  end

  def autolink_spec({:attribute, _, :opaque, ast}, _opts) do
    {name, _, args} = ast

    args =
      for arg <- args do
        {:var, _, name} = arg
        Atom.to_string(name)
      end
      |> Enum.intersperse(", ")

    IO.iodata_to_binary([Atom.to_string(name), "(", args, ")"])
  end

  def autolink_spec(ast, opts) do
    config = struct!(Autolink, opts)

    {name, quoted} =
      case ast do
        {:attribute, _, kind, {{name, _arity}, ast}} when kind in [:spec, :callback] ->
          {name, Enum.map(ast, &Code.Typespec.spec_to_quoted(name, &1))}

        {:attribute, _, :type, ast} ->
          {name, _, _} = ast
          {name, Code.Typespec.type_to_quoted(ast)}
      end

    formatted = format_spec(ast)
    autolink_spec(quoted, name, formatted, config)
  end

  @impl true
  def highlight_info() do
    %{
      language_name: "erlang",
      lexer: Makeup.Lexers.ErlangLexer,
      opts: []
    }
  end

  ## Shared between Erlang & Elixir

  @doc false
  def get_abstract_code(module) do
    case :code.get_object_code(module) do
      {^module, binary, _file} ->
        case :beam_lib.chunks(binary, [:abstract_code]) do
          {:ok, {_, [{:abstract_code, {_vsn, abstract_code}}]}} -> abstract_code
          _otherwise -> []
        end

      :error ->
        []
    end
  end

  @doc false
  def find_module_line(module, abst_code) do
    Enum.find_value(abst_code, fn
      {:attribute, anno, :module, ^module} -> anno_line(anno)
      _ -> nil
    end)
  end

  # Returns a map of {name, arity} => spec.
  def get_specs(module) do
    case Code.Typespec.fetch_specs(module) do
      {:ok, specs} -> Map.new(specs)
      :error -> %{}
    end
  end

  def get_callbacks(module) do
    case Code.Typespec.fetch_callbacks(module) do
      {:ok, callbacks} -> Map.new(callbacks)
      :error -> %{}
    end
  end

  ## Autolink

  defp walk_doc(list, config) when is_list(list) do
    Enum.map(list, &walk_doc(&1, config))
  end

  defp walk_doc(binary, _) when is_binary(binary) do
    binary
  end

  defp walk_doc({:a, attrs, inner, _meta} = ast, config) do
    case attrs[:rel] do
      "https://erlang.org/doc/link/seeerl" ->
        {fragment, url} = extract_fragment(attrs[:href] || "")

        case String.split(url, ":") do
          [module] ->
            autolink(:module, module, fragment, inner, config)

          [app, module] ->
            inner = strip_app(inner, app)
            autolink(:module, module, fragment, inner, config)

          _ ->
            warn_ref(attrs[:href], config)
            inner
        end

      "https://erlang.org/doc/link/seemfa" ->
        {kind, url} =
          case String.split(attrs[:href], "Module:") do
            [url] -> {:function, url}
            [left, right] -> {:callback, left <> right}
          end

        case String.split(url, ":") do
          [mfa] ->
            autolink(kind, mfa, "", inner, config)

          [app, mfa] ->
            inner = strip_app(inner, app)
            autolink(kind, mfa, "", inner, config)
        end

      "https://erlang.org/doc/link/seetype" ->
        case String.split(attrs[:href], ":") do
          [type] ->
            autolink(:type, type, "", inner, config)

          [app, type] ->
            inner = strip_app(inner, app)
            autolink(:type, type, "", inner, config)
        end

      "https://erlang.org/doc/link/" <> see ->
        # TODO: remove me before release
        unless System.get_env("SKIP_SEE_WARNING") do
          warn_ref(attrs[:href] <> " (#{see})", config)
        end

        inner

      _ ->
        ast
    end
  end

  defp walk_doc({tag, attrs, ast, meta}, config) do
    {tag, attrs, walk_doc(ast, config), meta}
  end

  defp extract_fragment(url) do
    case String.split(url, "#", parts: 2) do
      [url] -> {"", url}
      [url, fragment] -> {"#" <> fragment, url}
    end
  end

  defp strip_app([{:code, attrs, [code], meta}], app) do
    [{:code, attrs, strip_app(code, app), meta}]
  end

  defp strip_app(code, app) when is_binary(code) do
    String.trim_leading(code, "//#{app}/")
  end

  defp strip_app(other, _app) do
    other
  end

  defp warn_ref(href, config) do
    message = "invalid reference: #{href}"
    Autolink.maybe_warn(message, config, nil, %{})
  end

  defp autolink(kind, string, fragment, inner, config) do
    if url = url(kind, string, config) do
      {:a, [href: url <> fragment], inner, %{}}
    else
      inner
    end
  end

  defp url(:module, string, config) do
    ref = {:module, String.to_atom(string)}
    do_url(ref, string, config)
  end

  defp url(kind, string, config) do
    [module, name, arity] =
      case String.split(string, ["#", "/"]) do
        [module, name, arity] ->
          [module, name, arity]

        # this is what docgen_xml_to_chunk returns
        [module, name] when kind == :type ->
          # TODO: don't assume 0-arity, instead find first {:type, module, name, arity} ref
          # and use that arity.
          [module, name, "0"]
      end

    name = String.to_atom(name)
    arity = String.to_integer(arity)

    original_text =
      if kind == :type and arity == 0 do
        "#{name}()"
      else
        "#{name}/#{arity}"
      end

    if module == "" do
      ref = {kind, config.current_module, name, arity}
      visibility = Refs.get_visibility(ref)

      if visibility == :public do
        final_url({kind, name, arity}, config)
      else
        Autolink.maybe_warn(ref, config, visibility, %{original_text: original_text})
        nil
      end
    else
      ref = {kind, String.to_atom(module), name, arity}
      original_text = "#{module}:#{original_text}"
      do_url(ref, original_text, config)
    end
  end

  defp do_url(ref, original_text, config) do
    visibility = Refs.get_visibility(ref)

    # TODO: type with content = %{} in otp xml is marked as :hidden, it should be :public

    if visibility == :public or (visibility == :hidden and elem(ref, 0) == :type) do
      final_url(ref, config)
    else
      Autolink.maybe_warn(ref, config, visibility, %{original_text: original_text})
      nil
    end
  end

  defp final_url({:module, module}, config) do
    tool = Autolink.tool(module, config)
    Autolink.app_module_url(tool, module, config)
  end

  defp final_url({kind, name, arity}, _config) do
    fragment(:ex_doc, kind, name, arity)
  end

  defp final_url({kind, module, name, arity}, config) do
    tool = Autolink.tool(module, config)
    module_url = Autolink.app_module_url(tool, module, config)
    # TODO: fix me
    module_url = String.trim_trailing(module_url, "#content")
    module_url <> fragment(tool, kind, name, arity)
  end

  defp fragment(:otp, :function, name, arity) do
    "##{name}-#{arity}"
  end

  defp fragment(:otp, :callback, name, arity) do
    "#Module:#{name}-#{arity}"
  end

  defp fragment(:otp, :type, name, _arity) do
    "#type-#{name}"
  end

  defp fragment(:ex_doc, :function, name, arity) do
    "##{name}/#{arity}"
  end

  defp fragment(:ex_doc, :callback, name, arity) do
    "#c:#{name}/#{arity}"
  end

  defp fragment(:ex_doc, :type, name, arity) do
    "#t:#{name}/#{arity}"
  end

  # Traverses quoted and formatted string of the typespec AST, replacing refs with links.
  #
  # Let's say we have this typespec:
  #
  #     -spec f(X) -> #{atom() => bar(), integer() => X}.
  #
  # We traverse the AST and find types and their string representations:
  #
  #     -spec f(X) -> #{atom() => bar(), integer() => X}.
  #                     ^^^^      ^^^    ^^^^^^^
  #
  #     atom/0    => atom
  #     bar/0     => bar
  #     integer/0 => integer
  #
  # We then traverse the formatted string, *in order*, replacing the type strings with links:
  #
  #     "atom("    => "atom("
  #     "bar("     => "<a>bar</a>("
  #     "integer(" => "integer("
  #
  # Finally we end up with:
  #
  #     -spec f(X) -> #{atom() => <a>bar</a>(), integer() => X}.
  #
  # All of this hassle is to preserve the original *text layout* of the initial representation,
  # all the spaces, newlines, etc.
  defp autolink_spec(quoted, name, formatted, config) do
    acc =
      for quoted <- List.wrap(quoted) do
        {_quoted, acc} =
          Macro.prewalk(quoted, [], fn
            # module.name(args)
            {{:., _, [module, name]}, _, args}, acc ->
              {{:t, [], args}, [{pp({module, name}), {module, name, length(args)}} | acc]}

            {name, _, _}, acc when name in [:<<>>, :..] ->
              {nil, acc}

            # -1
            {:-, _, [int]}, acc when is_integer(int) ->
              {nil, acc}

            # fun() (spec_to_quoted expands it to (... -> any())
            {:->, _, [[{name, _, _}], {:any, _, _}]}, acc when name == :... ->
              {nil, acc}

            # #{x :: t()}
            {:field_type, _, [name, type]}, acc when is_atom(name) ->
              {type, acc}

            {name, _, args} = ast, acc when is_atom(name) and is_list(args) ->
              arity = length(args)

              cond do
                name in [:"::", :when, :%{}, :{}, :|, :->, :record] ->
                  {ast, acc}

                # %{required(...) => ..., optional(...) => ...}
                name in [:required, :optional] and arity == 1 ->
                  {ast, acc}

                # name(args)
                true ->
                  {ast, [{pp(name), {name, arity}} | acc]}
              end

            other, acc ->
              {other, acc}
          end)

        acc
        |> Enum.reverse()
        # drop the name of the typespec
        |> Enum.drop(1)
      end
      |> Enum.concat()

    put(acc)

    # Drop and re-add type name (it, the first element in acc, is dropped there too)
    #
    #     1. foo() :: bar()
    #     2.    () :: bar()
    #     3.    () :: <a>bar</a>()
    #     4. foo() :: <a>bar</a>()
    name = pp(name)
    formatted = trim_name(formatted, name)
    formatted = replace(formatted, acc, config)
    name <> formatted
  end

  defp trim_name(string, name) do
    name_size = byte_size(name)
    binary_part(string, name_size, byte_size(string) - name_size)
  end

  defp replace(formatted, [], _config) do
    formatted
  end

  defp replace(formatted, acc, config) do
    String.replace(formatted, Enum.map(acc, &"#{elem(&1, 0)}("), fn string ->
      string = String.trim_trailing(string, "(")
      {other, ref} = pop()

      if string != other do
        Autolink.maybe_warn(
          "internal inconsistency, please submit bug: #{inspect(string)} != #{inspect(other)}",
          config,
          nil,
          nil
        )
      end

      url =
        case ref do
          {name, arity} ->
            visibility = Refs.get_visibility({:type, config.current_module, name, arity})

            if visibility in [:public, :hidden] do
              final_url({:type, name, arity}, config)
            end

          {module, name, arity} ->
            ref = {:type, module, name, arity}
            visibility = Refs.get_visibility(ref)

            if visibility in [:public, :hidden] do
              final_url(ref, config)
            else
              original_text = "#{string}/#{arity}"
              Autolink.maybe_warn(ref, config, visibility, %{original_text: original_text})
              nil
            end
        end

      if url do
        ~s|<a href="#{url}">#{string}</a>(|
      else
        string <> "("
      end
    end)
  end

  defp put(items) do
    Process.put({__MODULE__, :stack}, items)
  end

  defp pop() do
    [head | tail] = Process.get({__MODULE__, :stack})
    put(tail)
    head
  end

  defp pp(name) when is_atom(name) do
    :io_lib.format("~p", [name]) |> IO.iodata_to_binary()
  end

  defp pp({module, name}) when is_atom(module) and is_atom(name) do
    :io_lib.format("~p:~p", [module, name]) |> IO.iodata_to_binary()
  end

  defp format_spec(ast) do
    {:attribute, _, type, _} = ast

    # `-type ` => 6
    offset = byte_size(Atom.to_string(type)) + 2

    options = [linewidth: 98 + offset]
    :erl_pp.attribute(ast, options) |> IO.iodata_to_binary() |> trim_offset(offset)
  end

  ## Helpers

  defp module_type(module) do
    cond do
      function_exported?(module, :behaviour_info, 1) ->
        :behaviour

      true ->
        :module
    end
  end

  # `-type t() :: atom()` becomes `t() :: atom().`
  defp trim_offset(binary, offset) do
    binary
    |> String.trim()
    |> String.split("\n")
    |> Enum.map(fn line ->
      binary_part(line, offset, byte_size(line) - offset)
    end)
    |> Enum.join("\n")
  end

  defp anno_line(line) when is_integer(line), do: abs(line)
  defp anno_line(anno), do: anno |> :erl_anno.line() |> abs()
end
