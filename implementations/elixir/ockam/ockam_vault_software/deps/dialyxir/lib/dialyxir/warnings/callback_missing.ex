defmodule Dialyxir.Warnings.CallbackMissing do
  @moduledoc """
  Module implements a behaviour, but does not have all of its
  callbacks. This is also a compiler warning.

  ## Example

      defmodule ExampleBehaviour do
        @callback ok() :: :ok
        @callback missing() :: :ok
      end

      defmodule Example do
        @behaviour ExampleBehaviour

        def ok() do
          :ok
        end
      end
  """

  @behaviour Dialyxir.Warning

  @impl Dialyxir.Warning
  @spec warning() :: :callback_missing
  def warning(), do: :callback_missing

  @impl Dialyxir.Warning
  @spec format_short([String.t()]) :: String.t()
  def format_short(args), do: format_long(args)

  @impl Dialyxir.Warning
  @spec format_long([String.t()]) :: String.t()
  def format_long([behaviour, function, arity]) do
    pretty_behaviour = Erlex.pretty_print(behaviour)

    "Undefined callback function #{function}/#{arity} (behaviour #{pretty_behaviour})."
  end

  @impl Dialyxir.Warning
  @spec explain() :: String.t()
  def explain() do
    @moduledoc
  end
end
