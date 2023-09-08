defmodule Dialyxir.Warnings.MapUpdate do
  @moduledoc """
  Elixir can only use the map update syntax to update a key that is in
  the map.

  ## Example

      defmodule Example do
        @spec error() :: map
        def error() do
          map = %{exists: :exists}
          %{map | does_not_exist: :fail}
        end
      end
  """

  @behaviour Dialyxir.Warning

  @impl Dialyxir.Warning
  @spec warning() :: :map_update
  def warning(), do: :map_update

  @impl Dialyxir.Warning
  @spec format_short([String.t()]) :: String.t()
  def format_short([_map, key]) do
    pretty_key = Erlex.pretty_print(key)
    "Attempted to update key #{pretty_key} in a map that does not have that key."
  end

  @impl Dialyxir.Warning
  @spec format_long([String.t()]) :: String.t()
  def format_long([map, key]) do
    pretty_key = Erlex.pretty_print(key)
    pretty_map = Erlex.pretty_print(map)

    """
    Attempted to update a key in a map that does not have that key.

    Key:
    #{pretty_key}

    Map:
    #{pretty_map}
    """
  end

  @impl Dialyxir.Warning
  @spec explain() :: String.t()
  def explain() do
    @moduledoc
  end
end
