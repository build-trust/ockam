defmodule Dialyxir.Warnings.CallbackSpecTypeMismatch do
  @moduledoc """
  The return type in the @spec does not match the
  expected return type of the behaviour.

  ## Example

      defmodule ExampleBehaviour do
        @callback ok(:ok) :: :ok
      end

      defmodule Example do
        @behaviour ExampleBehaviour

        @spec ok(:ok) :: :error
        def ok(:ok) do
          :error
        end
      end
  """

  @behaviour Dialyxir.Warning

  @impl Dialyxir.Warning
  @spec warning() :: :callback_spec_type_mismatch
  def warning(), do: :callback_spec_type_mismatch

  @impl Dialyxir.Warning
  @spec format_short([String.t()]) :: String.t()
  def format_short([_behaviour, function | _]) do
    "The @spec return type does not match the expected return type for #{function}."
  end

  @impl Dialyxir.Warning
  @spec format_long([String.t()]) :: String.t()
  def format_long([behaviour, function, arity, success_type, callback_type]) do
    pretty_behaviour = Erlex.pretty_print(behaviour)
    pretty_success_type = Erlex.pretty_print_type(success_type)
    pretty_callback_type = Erlex.pretty_print_type(callback_type)

    """
    The @spec return type does not match the expected return type
    for #{function}/#{arity} callback in #{pretty_behaviour} behaviour.

    Actual:
    @spec #{function}(...) :: #{pretty_success_type}

    Expected:
    @spec #{function}(...) :: #{pretty_callback_type}
    """
  end

  @impl Dialyxir.Warning
  @spec explain() :: String.t()
  def explain() do
    @moduledoc
  end
end
