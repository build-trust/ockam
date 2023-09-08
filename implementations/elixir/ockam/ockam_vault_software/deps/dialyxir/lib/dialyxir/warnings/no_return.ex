defmodule Dialyxir.Warnings.NoReturn do
  @moduledoc """
  The function has no return. This is usually due to an issue later on
  in the call stack causing it to not be recognized as returning for
  some reason. It is often helpful to cross reference the complete
  list of warnings with the call stack in the function and fix the
  deepest part of the call stack, which will usually fix many of the
  other no_return errors.

  ## Example

      defmodule Example do
        def ok() do
          Enum.each([1, 2, 3], fn _ -> raise "error" end)
        end
      end

    or

      defmodule Example do
        def ok() do
          raise "error"

          :ok
        end

        def ok(:ok) do
          ok()
        end
      end
  """

  @behaviour Dialyxir.Warning

  @impl Dialyxir.Warning
  @spec warning() :: :no_return
  def warning(), do: :no_return

  @impl Dialyxir.Warning
  @spec format_short([String.t()]) :: String.t()
  def format_short(args), do: format_long(args)

  @impl Dialyxir.Warning
  @spec format_long([String.t() | atom]) :: String.t()
  def format_long([type | name]) do
    name_string =
      case name do
        [] ->
          "The created anonymous function"

        [function, arity] ->
          "Function #{function}/#{arity}"
      end

    type_string =
      case type do
        :no_match ->
          "has no clauses that will ever match."

        :only_explicit ->
          "only terminates with explicit exception."

        :only_normal ->
          "has no local return."

        :both ->
          "has no local return."
      end

    "#{name_string} #{type_string}"
  end

  @impl Dialyxir.Warning
  @spec explain() :: String.t()
  def explain() do
    @moduledoc
  end
end
