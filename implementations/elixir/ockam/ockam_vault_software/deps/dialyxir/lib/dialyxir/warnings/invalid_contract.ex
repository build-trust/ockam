defmodule Dialyxir.Warnings.InvalidContract do
  @moduledoc """
  The @spec for the function does not match the success typing of the
  function.

  ## Example

      defmodule Example do
        @spec process(:error) :: :ok
        def process(:ok) do
          :ok
        end
      end

  The @spec in this case claims that the function accepts a parameter
  :error but the function head only accepts :ok, resulting in the
  mismatch.
  """

  @behaviour Dialyxir.Warning

  @impl Dialyxir.Warning
  @spec warning() :: :invalid_contract
  def warning(), do: :invalid_contract

  @impl Dialyxir.Warning
  @spec format_short([String.t()]) :: String.t()
  def format_short([_module, function | _]) do
    "Invalid type specification for function #{function}."
  end

  @impl Dialyxir.Warning
  @spec format_long([String.t()]) :: String.t()
  def format_long([module, function, arity, signature]) do
    pretty_module = Erlex.pretty_print(module)
    pretty_signature = Erlex.pretty_print_contract(signature)

    """
    The @spec for the function does not match the success typing of the function.

    Function:
    #{pretty_module}.#{function}/#{arity}

    Success typing:
    @spec #{function}#{pretty_signature}
    """
  end

  @impl Dialyxir.Warning
  @spec explain() :: String.t()
  def explain() do
    @moduledoc
  end
end
