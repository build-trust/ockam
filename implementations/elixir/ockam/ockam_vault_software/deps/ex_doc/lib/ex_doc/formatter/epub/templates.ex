defmodule ExDoc.Formatter.EPUB.Templates do
  @moduledoc false

  require EEx

  alias ExDoc.Formatter.HTML
  alias ExDoc.Formatter.HTML.Templates, as: H

  @doc """
  Generate content from the module template for a given `node`
  """
  def module_page(config, module_node) do
    summary = H.module_summary(module_node)
    module_template(config, module_node, summary)
  end

  @doc """
  Generated ID for static file
  """
  def static_file_to_id(static_file) do
    prefix = static_file |> HTML.filename_to_title() |> HTML.text_to_id()
    extension = static_file |> Path.extname() |> String.replace_prefix(".", "-")

    "#{prefix}#{extension}"
  end

  @doc """
  Creates the Package Document Definition.

  this definition encapsulates the publication metadata and the resource
  information that constitute the EPUB publication. This definition also
  includes the default reading order.

  See http://www.idpf.org/epub/30/spec/epub30-publications.html#sec-package-def.
  """
  EEx.function_from_file(
    :def,
    :content_template,
    Path.expand("templates/content_template.eex", __DIR__),
    [:config, :nodes, :uuid, :datetime, :static_files],
    trim: true
  )

  @doc """
  Creates a chapter which contains all the details about an individual module.

  This chapter can include the following sections: *functions*, *types*, *callbacks*.
  """
  EEx.function_from_file(
    :def,
    :module_template,
    Path.expand("templates/module_template.eex", __DIR__),
    [:config, :module, :summary],
    trim: true
  )

  @doc """
  Creates the table of contents.

  This template follows the EPUB Navigation Document Definition.

  See http://www.idpf.org/epub/30/spec/epub30-contentdocs.html#sec-xhtml-nav.
  """
  EEx.function_from_file(
    :def,
    :nav_template,
    Path.expand("templates/nav_template.eex", __DIR__),
    [:config, :nodes],
    trim: true
  )

  @doc """
  Creates a new chapter when the user provides additional files.
  """
  EEx.function_from_file(
    :def,
    :extra_template,
    Path.expand("templates/extra_template.eex", __DIR__),
    [:config, :title, :title_content, :content],
    trim: true
  )

  @doc """
  Creates the cover page for the EPUB document.
  """
  EEx.function_from_file(
    :def,
    :title_template,
    Path.expand("templates/title_template.eex", __DIR__),
    [:config],
    trim: true
  )

  EEx.function_from_file(
    :defp,
    :head_template,
    Path.expand("templates/head_template.eex", __DIR__),
    [:config, :page],
    trim: true
  )

  EEx.function_from_file(
    :defp,
    :nav_item_template,
    Path.expand("templates/nav_item_template.eex", __DIR__),
    [:name, :nodes],
    trim: true
  )

  EEx.function_from_file(
    :defp,
    :toc_item_template,
    Path.expand("templates/toc_item_template.eex", __DIR__),
    [:nodes],
    trim: true
  )

  "templates/media-types.txt"
  |> Path.expand(__DIR__)
  |> File.read!()
  |> String.split("\n", trim: true)
  |> Enum.each(fn line ->
    [extension, media] = String.split(line, ",")

    defp media_type("." <> unquote(extension)) do
      unquote(media)
    end
  end)

  defp media_type(arg),
    do: raise("asset with extension #{inspect(arg)} is not supported by EPUB format")
end
