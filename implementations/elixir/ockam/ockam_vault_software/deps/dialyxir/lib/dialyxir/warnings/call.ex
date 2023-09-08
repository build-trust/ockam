defmodule Dialyxir.Warnings.Call do
  @moduledoc """
  The function call will not succeed.

  ## Example

      defmodule Example do
        def ok() do
          ok(:error)
        end

        def ok(:ok) do
          :ok
        end
      end
  """

  @behaviour Dialyxir.Warning

  @impl Dialyxir.Warning
  @spec warning() :: :call
  def warning(), do: :call

  @impl Dialyxir.Warning
  @spec format_short([String.t()]) :: String.t()
  def format_short([_module, function | _]) do
    "The function call #{function} will not succeed."
  end

  @impl Dialyxir.Warning
  @spec format_long([String.t()]) :: String.t()
  def format_long([
        module,
        function,
        args,
        arg_positions,
        fail_reason,
        signature_args,
        signature_return,
        contract
      ]) do
    pretty_args = Erlex.pretty_print_args(args)
    pretty_module = Erlex.pretty_print(module)

    call_string =
      Dialyxir.WarningHelpers.call_or_apply_to_string(
        arg_positions,
        fail_reason,
        signature_args,
        signature_return,
        contract
      )

    """
    The function call will not succeed.

    #{pretty_module}.#{function}#{pretty_args}

    #{String.trim_trailing(call_string)}
    """
  end

  @impl Dialyxir.Warning
  @spec explain() :: String.t()
  def explain() do
    @moduledoc
  end
end
