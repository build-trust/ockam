defmodule Dialyxir.Warnings.GuardFailPattern do
  @moduledoc """
  The clause guard describes a condition of literals that fails the pattern
  given in the function head.

  ## Example

      defmodule Example do
        def ok(n = 0) when not n < 1 do
          :ok
        end
      end
  """

  @behaviour Dialyxir.Warning

  @impl Dialyxir.Warning
  @spec warning() :: :guard_fail_pat
  def warning(), do: :guard_fail_pat

  @impl Dialyxir.Warning
  @spec format_short([String.t()]) :: String.t()
  def format_short([pattern, _]) do
    "The clause guard #{pattern} cannot succeed."
  end

  @impl Dialyxir.Warning
  @spec format_long([String.t()]) :: String.t()
  def format_long([pattern, type]) do
    pretty_type = Erlex.pretty_print_type(type)
    pretty_pattern = Erlex.pretty_print_pattern(pattern)

    "The clause guard cannot succeed. The pattern #{pretty_pattern} " <>
      "was matched against the type #{pretty_type}."
  end

  @impl Dialyxir.Warning
  @spec explain() :: String.t()
  def explain() do
    @moduledoc
  end
end
