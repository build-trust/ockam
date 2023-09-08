defmodule Dialyxir.Warnings.PatternMatch do
  @moduledoc """
  The pattern matching is never given a value that satisfies all of
  its clauses.

  ## Example

      defmodule Example do
        def ok() do
          unmatched(:ok)
        end

        defp unmatched(:ok), do: :ok

        defp unmatched(:error), do: :error
      end
  """

  @behaviour Dialyxir.Warning

  @impl Dialyxir.Warning
  @spec warning() :: :pattern_match
  def warning(), do: :pattern_match

  @impl Dialyxir.Warning
  @spec format_short([String.t()]) :: String.t()
  def format_short([_pattern, type]) do
    pretty_type = Erlex.pretty_print_type(type)
    "The pattern can never match the type #{pretty_type}."
  end

  @impl Dialyxir.Warning
  @spec format_long([String.t()]) :: String.t()
  def format_long([pattern, type]) do
    pretty_pattern = Erlex.pretty_print_pattern(pattern)
    pretty_type = Erlex.pretty_print_type(type)

    """
    The pattern can never match the type.

    Pattern:
    #{pretty_pattern}

    Type:
    #{pretty_type}
    """
  end

  @impl Dialyxir.Warning
  @spec explain() :: String.t()
  def explain() do
    @moduledoc
  end
end
