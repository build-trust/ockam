defmodule Dialyxir.Warnings.MissingRange do
  @moduledoc """
  Function spec declares a list of types, but function returns value
  outside stated range.

  This error only appears with the :overspecs flag.

  ## Example

      defmodule Example do
        @spec foo(any()) :: :ok
        def foo(:ok) do
          :ok
        end

        def foo(_) do
          :error
        end
      end
  """

  @behaviour Dialyxir.Warning

  @impl Dialyxir.Warning
  @spec warning() :: :missing_range
  def warning(), do: :missing_range

  @impl Dialyxir.Warning
  @spec format_short([String.t()]) :: String.t()
  def format_short([module, function, arity | _]) do
    pretty_module = Erlex.pretty_print(module)

    "The type specification is missing types returned by #{pretty_module}.#{function}/#{arity}."
  end

  @impl Dialyxir.Warning
  @spec format_long([String.t()]) :: String.t()
  def format_long([module, function, arity, extra_ranges, contract_range]) do
    pretty_module = Erlex.pretty_print(module)
    pretty_contract_range = Erlex.pretty_print_type(contract_range)
    pretty_extra_ranges = Erlex.pretty_print_type(extra_ranges)

    """
    The type specification is missing types returned by function.

    Function:
    #{pretty_module}.#{function}/#{arity}

    Type specification return types:
    #{pretty_contract_range}

    Missing from spec:
    #{pretty_extra_ranges}
    """
  end

  @impl Dialyxir.Warning
  @spec explain() :: String.t()
  def explain() do
    @moduledoc
  end
end
