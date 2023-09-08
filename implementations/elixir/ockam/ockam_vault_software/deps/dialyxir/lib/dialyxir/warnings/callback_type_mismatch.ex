defmodule Dialyxir.Warnings.CallbackTypeMismatch do
  @moduledoc """
  The success type of the function does not match the callback type in
  the behaviour.

  ## Example

      defmodule ExampleBehaviour do
        @callback ok() :: :ok
      end

      defmodule Example do
        @behaviour ExampleBehaviour

        def ok() do
          :error
        end
      end
  """

  @behaviour Dialyxir.Warning

  @impl Dialyxir.Warning
  @spec warning() :: :callback_type_mismatch
  def warning(), do: :callback_type_mismatch

  @impl Dialyxir.Warning
  @spec format_short([String.t()]) :: String.t()
  def format_short([_behaviour, function | _]) do
    "Type mismatch for @callback #{function}."
  end

  @impl Dialyxir.Warning
  @spec format_long([String.t() | non_neg_integer]) :: String.t()
  def format_long([behaviour, function, arity, fail_type, success_type]) do
    pretty_behaviour = Erlex.pretty_print(behaviour)
    pretty_fail_type = Erlex.pretty_print_type(fail_type)
    pretty_success_type = Erlex.pretty_print_type(success_type)

    """
    Type mismatch for @callback #{function}/#{arity} in #{pretty_behaviour} behaviour.

    Expected type:
    #{pretty_success_type}

    Actual type:
    #{pretty_fail_type}
    """
  end

  @impl Dialyxir.Warning
  @spec explain() :: String.t()
  def explain() do
    @moduledoc
  end
end
