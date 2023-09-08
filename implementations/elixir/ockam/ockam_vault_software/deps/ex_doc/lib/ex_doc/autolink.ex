defmodule ExDoc.Autolink do
  @moduledoc false

  # * `:apps` - the apps that the docs are being generated for. When linking modules they are
  #   checked if they are part of the app and based on that the links are relative or absolute.
  #
  # * `:current_module` - the module that the docs are being generated for. Used to link local
  #   calls and see if remote calls are in the same app.
  #
  # * `:module_id` - id of the module being documented (e.g.: `"String"`)
  #
  # * `:file` - source file location
  #
  # * `:line` - line number of the beginning of the documentation
  #
  # * `:id` - a module/function/etc being documented (e.g.: `"String.upcase/2"`)
  #
  # * `:ext` - the extension (`".html"`, "`.xhtml"`, etc)
  #
  # * `:extras` - list of extras
  #
  # * `:skip_undefined_reference_warnings_on` - list of modules to skip the warning on

  defstruct [
    :current_module,
    :module_id,
    :id,
    :line,
    file: "nofile",
    apps: [],
    extras: [],
    deps: [],
    ext: ".html",
    siblings: [],
    skip_undefined_reference_warnings_on: []
  ]

  @hexdocs "https://hexdocs.pm/"
  @otpdocs "https://erlang.org/doc/man/"

  def app_module_url(:ex_doc, module, %{current_module: module} = config) do
    path = module |> inspect() |> String.trim_leading(":")
    ex_doc_app_url(module, config, path, config.ext, "#content")
  end

  def app_module_url(:ex_doc, module, config) do
    path = module |> inspect() |> String.trim_leading(":")
    ex_doc_app_url(module, config, path, config.ext, "")
  end

  def app_module_url(:otp, module, _config) do
    @otpdocs <> "#{module}.html"
  end

  def app_module_url(:no_tool, _, _) do
    nil
  end

  # TODO: make more generic
  @doc false
  def ex_doc_app_url(module, config, path, ext, suffix) do
    if app = app(module) do
      if app in config.apps do
        path <> ext <> suffix
      else
        config.deps
        |> Keyword.get_lazy(app, fn -> @hexdocs <> "#{app}" end)
        |> String.trim_trailing("/")
        |> Kernel.<>("/" <> path <> ".html" <> suffix)
      end
    else
      path <> ext <> suffix
    end
  end

  defp app(module) do
    {_, app} = app_info(module)
    app
  end

  @doc false
  def tool(module, config) do
    if match?("Elixir." <> _, Atom.to_string(module)) do
      :ex_doc
    else
      {otp, app} = app_info(module)
      apps = Enum.uniq(config.apps ++ Keyword.keys(config.deps))

      if otp == true and app not in apps do
        :otp
      else
        :ex_doc
      end
    end
  end

  defp app_info(module) do
    case :code.which(module) do
      :preloaded ->
        {true, :erts}

      maybe_path ->
        otp? = is_list(maybe_path) and List.starts_with?(maybe_path, :code.lib_dir())

        app =
          case :application.get_application(module) do
            {:ok, app} ->
              app

            _ ->
              with true <- is_list(maybe_path),
                   [_, "ebin", app, "lib" | _] <- maybe_path |> Path.split() |> Enum.reverse() do
                String.to_atom(app)
              else
                _ -> nil
              end
          end

        {otp?, app}
    end
  end

  def maybe_warn(ref, config, visibility, metadata) do
    skipped = config.skip_undefined_reference_warnings_on
    file = Path.relative_to(config.file, File.cwd!())
    line = config.line

    unless Enum.any?([config.id, config.module_id, file], &(&1 in skipped)) do
      warn(ref, {file, line}, config.id, visibility, metadata)
    end
  end

  defp warn(message, {file, line}, id) do
    warning = IO.ANSI.format([:yellow, "warning: ", :reset])

    stacktrace =
      "  #{file}" <>
        if(line, do: ":#{line}", else: "") <>
        if(id, do: ": #{id}", else: "")

    IO.puts(:stderr, [warning, message, ?\n, stacktrace, ?\n])
  end

  defp warn(ref, file_line, id, visibility, metadata)

  defp warn(
         {:module, _module},
         {file, line},
         id,
         visibility,
         %{mix_task: true, original_text: original_text}
       ) do
    message =
      "documentation references \"#{original_text}\" but it is " <>
        format_visibility(visibility, :module)

    warn(message, {file, line}, id)
  end

  defp warn(
         {:module, _module},
         {file, line},
         id,
         visibility,
         %{original_text: original_text}
       ) do
    message =
      "documentation references module \"#{original_text}\" but it is " <>
        format_visibility(visibility, :module)

    warn(message, {file, line}, id)
  end

  defp warn(
         nil,
         {file, line},
         id,
         _visibility,
         %{file_path: _file_path, original_text: original_text}
       ) do
    message = "documentation references file \"#{original_text}\" but it does not exist"

    warn(message, {file, line}, id)
  end

  defp warn(
         {kind, _module, _name, _arity},
         {file, line},
         id,
         visibility,
         %{original_text: original_text}
       ) do
    message =
      "documentation references #{kind} \"#{original_text}\" but it is " <>
        format_visibility(visibility, kind)

    warn(message, {file, line}, id)
  end

  defp warn(message, {file, line}, id, _, _) when is_binary(message) do
    warn(message, {file, line}, id)
  end

  # there is not such a thing as private callback or private module
  defp format_visibility(visibility, kind) when kind in [:module, :callback], do: "#{visibility}"

  # typep is defined as :hidden, since there is no :private visibility value
  # but type defined with @doc false also is the stored the same way.
  defp format_visibility(:hidden, :type), do: "hidden or private"

  # for the rest, it can either be undefined or private
  defp format_visibility(:undefined, _kind), do: "undefined or private"
  defp format_visibility(visibility, _kind), do: "#{visibility}"
end
