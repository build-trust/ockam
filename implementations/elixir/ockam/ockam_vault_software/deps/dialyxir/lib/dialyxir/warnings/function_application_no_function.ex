defmodule Dialyxir.Warnings.FunctionApplicationNoFunction do
  @moduledoc """
  The function being invoked exists, but has an arity mismatch.

  ## Example

      defmodule Example do
        def ok() do
          fun = fn _ -> :ok end
          fun.()
        end
      end
  """

  @behaviour Dialyxir.Warning

  @impl Dialyxir.Warning
  @spec warning() :: :fun_app_no_fun
  def warning(), do: :fun_app_no_fun

  @impl Dialyxir.Warning
  @spec format_short([String.t()]) :: String.t()
  def format_short([_, _, arity]) do
    "Function application will fail, because anonymous function has arity of #{arity}."
  end

  @impl Dialyxir.Warning
  @spec format_long([String.t()]) :: String.t()
  def format_long([op, type, arity]) do
    pretty_op = Erlex.pretty_print(op)
    pretty_type = Erlex.pretty_print_type(type)

    "Function application will fail, because #{pretty_op} :: #{pretty_type} is not a function of arity #{
      arity
    }."
  end

  @impl Dialyxir.Warning
  @spec explain() :: String.t()
  def explain() do
    @moduledoc
  end
end
