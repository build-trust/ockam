defmodule Makeup.Registry do
  @moduledoc """
  A registry that allows users to dynamically register new makeup lexers.

  Lexers should register themselves on application start.
  That way, you can add support for new programming languages by depending on the relevant lexers.
  This is useful for projects such as ExDoc, which might contain code
  in a number of different programming languages.
  """

  @name_registry_key :lexer_name_registry

  @extension_registry_key :lexer_extension_registry

  # --------------------------------------------------------------------------
  # Public API
  # --------------------------------------------------------------------------

  @doc """
  Gets the list of supported language names.
  """
  def supported_language_names() do
    Map.keys(get_name_registry())
  end

  @doc """
  Gets the list of supported language extensions.
  """
  def supported_file_extensions() do
    Map.keys(get_extension_registry())
  end

  @doc """
  Adds a new lexer to Makeup's registry under the given `name`.

  This function expects a language name (e.g. `"elixir"`) and a pair containing
  a `lexer` and a list of `options`.

  You might want to use the `Makeup.Registry.register_lexer/2` function instead.

  ## Examples

      alias Makeup.Lexers.ElixirLexer
      alias Makeup.Registry

      Registry.register_lexer_with_name("elixir", {ElixirLexer, []})
      Registry.register_lexer_with_name("iex", {ElixirLexer, []})
  """
  def register_lexer_with_name(name, {lexer, options}) when is_binary(name) do
    old_registry = get_name_registry()
    updated_registry = Map.put(old_registry, name, {lexer, options})
    put_name_registry(updated_registry)
  end

  @doc """
  Adds a new lexer to Makeup's registry under the given `extension`.

  This function expects a file extension (e.g. `"ex"`) and a pair containing
  a `lexer` and a list of `options`.

  You might want to use the `Makeup.Registry.register_lexer/2` function instead.

  ## Examples

      alias Makeup.Lexers.ElixirLexer
      alias Makeup.Registry

      Registry.register_lexer_with_extension("ex"), {ElixirLexer, []})
      Registry.register_lexer_with_extension("exs"), {ElixirLexer, []})
  """
  def register_lexer_with_extension(name, {lexer, options}) when is_binary(name) do
    old_registry = get_extension_registry()
    updated_registry = Map.put(old_registry, name, {lexer, options})
    put_extension_registry(updated_registry)
  end

  @doc """
  Add a new lexer to Makeup's registry under the given names and extensions.

  Expects a lexer `lexer` and a number of options:

    * `:options` (default: `[]`) - the lexer options.
      If your lexer doesn't take any options, you'll want the default value of `[]`.

    * `:names` (default: `[]`) - a list of strings with the language names for the lexer.
      Language names are strings, not atoms.
      Even if there is only one valid name, you must supply a list with that name.
      To avoid filling the registry unnecessarily, you should normalize your language names
      to lowercase strings.
      If the caller wants to support upper case language names for some reason,
      they can normalize the language names themselves.

    * `:extensions` (default: `[]`) - the list of file extensions for the languages supported by the lexer.
      For example, the elixir lexer should support the `"ex"` and `"exs"` file extensions.
      The extensions should not include the dot.
      That is, you should register `"ex"` and not `".ex"`.
      Even if there is only a supported extension, you must supply a list.

  ## Example

      alias Makeup.Registry
      alias Makeup.Lexers.ElixirLexer
      # The `:options` key is not required
      Registry.register_lexer(ElixirLexer, names: ["elixir", "iex"], extensions: ["ex", "exs"])

  """
  def register_lexer(lexer, opts) do
    options = Keyword.get(opts, :options, [])
    names = Keyword.get(opts, :names, [])
    extensions = Keyword.get(opts, :extensions, [])
    # Associate the lexer with the names
    for name <- names, do: register_lexer_with_name(name, {lexer, options})
    # Associate the lexer with the extensions
    for extension <- extensions, do: register_lexer_with_extension(extension, {lexer, options})
  end

  @doc """
  Fetches the lexer from Makeup's registry with the given `name`.

  Returns either `{:ok, {lexer, options}}` or `:error`.
  This behaviour is based on `Map.fetch/2`.
  """
  def fetch_lexer_by_name(name) do
    Map.fetch(get_name_registry(), name)
  end

  @doc """
  Fetches the lexer from Makeup's registry with the given `name`.

  Returns either `{lexer, options}` or raises a `KeyError`.
  This behaviour is based on `Map.fetch!/2`.
  """
  def fetch_lexer_by_name!(name) do
    Map.fetch!(get_name_registry(), name)
  end

  @doc """
  Gets the lexer from Makeup's registry with the given `name`.

  Returns either `{lexer, options}` or the `default` value
  (which by default is `nil`).
  This behaviour is based on `Map.get/3`.
  """
  def get_lexer_by_name(name, default \\ nil) do
    Map.get(get_name_registry(), name, default)
  end

  @doc """
  Fetches a lexer from Makeup's registry with the given file `extension`.

  Returns either `{:ok, {lexer, options}}` or `:error`.
  This behaviour is based on `Map.fetch/2`.
  """
  def fetch_lexer_by_extension(name) do
    Map.fetch(get_extension_registry(), name)
  end

  @doc """
  Fetches the lexer from Makeup's registry with the given file `extension`.

  Returns either `{:ok, {lexer, options}}` or raises a `KeyError`.
  This behaviour is based on `Map.fetch/2`.
  """
  def fetch_lexer_by_extension!(name) do
    Map.fetch!(get_extension_registry(), name)
  end

  @doc """
  Gets the lexer from Makeup's registry with the given file `extension`.

  Returns either `{lexer, options}` or the `default` value
  (which by default is `nil`).
  This behaviour is based on `Map.get/3`.
  """
  def get_lexer_by_extension(name, default \\ nil) do
    Map.get(get_extension_registry(), name, default)
  end

  # ---------------------------------------------------------------------------
  # Functions not meant to be used outside Makeup
  # ---------------------------------------------------------------------------
  # This functions are meant to be run on application startup
  # or to be used as helpers in Makeup's internal tests.
  # They are not meant to be invoked by users of Makeup

  @doc false
  def create_name_registry() do
    Application.put_env(:makeup, @name_registry_key, %{})
  end

  @doc false
  def create_extension_registry() do
    Application.put_env(:makeup, @extension_registry_key, %{})
  end

  # The `clean_*_registry` are actually the same as the `create_*_registry`,
  # but that's because of implementation details, so it makes sense to have
  # separate groups of functions

  @doc false
  def clean_name_registry() do
    put_name_registry(%{})
  end

  @doc false
  def clean_extension_registry() do
    put_extension_registry(%{})
  end

  # ----------------------------------------------------------------------------
  # Private helper functions
  # ----------------------------------------------------------------------------

  defp get_name_registry() do
    Application.get_env(:makeup, @name_registry_key)
  end

  defp put_name_registry(registry) do
    Application.put_env(:makeup, @name_registry_key, registry)
  end

  defp get_extension_registry() do
    Application.get_env(:makeup, @extension_registry_key)
  end

  defp put_extension_registry(registry) do
    Application.put_env(:makeup, @extension_registry_key, registry)
  end
end
