defmodule Dialyxir.Warnings.CallbackArgumentTypeMismatch do
  @moduledoc """
  Type of argument does not match the callback's expected type.

  ## Example

      defmodule ExampleBehaviour do
        @callback ok(:ok) :: :ok
      end

      defmodule Example do

        @behaviour ExampleBehaviour

        def ok(:error) do
          :ok
        end
    end
  """
  @behaviour Dialyxir.Warning

  @impl Dialyxir.Warning
  @spec warning() :: :callback_arg_type_mismatch
  def warning(), do: :callback_arg_type_mismatch

  @impl Dialyxir.Warning
  @spec format_short([String.t()]) :: String.t()
  def format_short([_behaviour, function, arity | _]) do
    "Type mismatch with behaviour callback to #{function}/#{arity}."
  end

  @impl Dialyxir.Warning
  @spec format_long([String.t()]) :: String.t()
  def format_long([behaviour, function, arity, position, success_type, callback_type]) do
    pretty_behaviour = Erlex.pretty_print(behaviour)
    pretty_success_type = Erlex.pretty_print_type(success_type)
    pretty_callback_type = Erlex.pretty_print_type(callback_type)
    ordinal_position = Dialyxir.WarningHelpers.ordinal(position)

    """
    The inferred type for the #{ordinal_position} argument is not a
    supertype of the expected type for the #{function}/#{arity} callback
    in the #{pretty_behaviour} behaviour.

    Success type:
    #{pretty_success_type}

    Behaviour callback type:
    #{pretty_callback_type}
    """
  end

  @impl Dialyxir.Warning
  @spec explain() :: String.t()
  def explain() do
    @moduledoc
  end
end
