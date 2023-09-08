defmodule ExDoc do
  @moduledoc false
  @ex_doc_version Mix.Project.config()[:version]

  alias ExDoc.Config

  @doc """
  Returns the ExDoc version (used in templates).
  """
  @spec version :: String.t()
  def version, do: @ex_doc_version

  @doc """
  Generates documentation for the given `project`, `vsn` (version)
  and `options`.
  """
  @spec generate_docs(String.t(), String.t(), Keyword.t()) :: atom
  def generate_docs(project, vsn, options)
      when is_binary(project) and is_binary(vsn) and is_list(options) do
    config = build_config(project, vsn, options)

    if processor = options[:markdown_processor] do
      ExDoc.Markdown.put_markdown_processor(processor)
    end

    docs = config.retriever.docs_from_dir(config.source_beam, config)
    find_formatter(config.formatter).run(docs, config)
  end

  @doc false
  @spec build_config(String.t(), String.t(), Keyword.t()) :: ExDoc.Config.t()
  def build_config(project, vsn, options) do
    {output, options} = Keyword.pop(options, :output, "./doc")
    {groups_for_modules, options} = Keyword.pop(options, :groups_for_modules, [])
    {nest_modules_by_prefix, options} = Keyword.pop(options, :nest_modules_by_prefix, [])
    {proglang, options} = Keyword.pop(options, :proglang, :elixir)

    {source_url_pattern, options} =
      Keyword.pop_lazy(options, :source_url_pattern, fn ->
        guess_url(options[:source_url], options[:source_ref] || ExDoc.Config.default_source_ref())
      end)

    preconfig = %Config{
      project: project,
      version: vsn,
      main: options[:main],
      output: normalize_output(output),
      homepage_url: options[:homepage_url],
      proglang: normalize_proglang(proglang),
      source_url_pattern: source_url_pattern,
      nest_modules_by_prefix: normalize_nest_modules_by_prefix(nest_modules_by_prefix),
      groups_for_modules: normalize_groups_for_modules(groups_for_modules)
    }

    struct(preconfig, options)
  end

  # Short path for programmatic interface
  defp find_formatter(modname) when is_atom(modname), do: modname

  defp find_formatter("ExDoc.Formatter." <> _ = name) do
    [name]
    |> Module.concat()
    |> check_formatter_module(name)
  end

  defp find_formatter(name) do
    [ExDoc.Formatter, String.upcase(name)]
    |> Module.concat()
    |> check_formatter_module(name)
  end

  defp check_formatter_module(modname, argname) do
    if Code.ensure_loaded?(modname) do
      modname
    else
      raise "formatter module #{inspect(argname)} not found"
    end
  end

  # Helpers

  defp normalize_output(output) do
    String.trim_trailing(output, "/")
  end

  defp normalize_proglang(binary) when is_binary(binary) do
    binary |> String.to_atom() |> normalize_proglang()
  end

  defp normalize_proglang(proglang) when proglang in [:elixir, :erlang] do
    proglang
  end

  defp normalize_proglang(proglang) do
    raise ArgumentError, "#{inspect(proglang)} is not supported"
  end

  defp normalize_groups_for_modules(groups_for_modules) do
    default_groups = [Deprecated: &deprecated?/1, Exceptions: &exception?/1]

    groups_for_modules ++
      Enum.reject(default_groups, fn {k, _} -> Keyword.has_key?(groups_for_modules, k) end)
  end

  defp deprecated?(%{deprecated: deprecated}), do: is_binary(deprecated)
  defp exception?(%{type: type}), do: type == :exception

  defp normalize_nest_modules_by_prefix(nest_modules_by_prefix) do
    nest_modules_by_prefix
    |> Enum.map(&inspect_atoms/1)
    |> Enum.sort()
    |> Enum.reverse()
  end

  defp inspect_atoms(atom) when is_atom(atom), do: inspect(atom)
  defp inspect_atoms(binary) when is_binary(binary), do: binary

  defp guess_url(url, ref) do
    with {:ok, host_with_path} <- http_or_https(url),
         {:ok, pattern} <- known_pattern(host_with_path, ref) do
      "https://" <> append_slash(host_with_path) <> pattern
    else
      _ -> url
    end
  end

  defp http_or_https("http://" <> rest),
    do: {:ok, rest}

  defp http_or_https("https://" <> rest),
    do: {:ok, rest}

  defp http_or_https(_),
    do: :error

  defp known_pattern("github.com/" <> _, ref),
    do: {:ok, "blob/#{ref}/%{path}#L%{line}"}

  defp known_pattern("gitlab.com/" <> _, ref),
    do: {:ok, "blob/#{ref}/%{path}#L%{line}"}

  defp known_pattern("bitbucket.org/" <> _, ref),
    do: {:ok, "src/#{ref}/%{path}#cl-%{line}"}

  defp known_pattern(_host_with_path, _ref),
    do: :error

  defp append_slash(url) do
    if :binary.last(url) == ?/, do: url, else: url <> "/"
  end
end
