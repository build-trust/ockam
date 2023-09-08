defmodule Dialyxir.Warnings.CallWithoutOpaque do
  @moduledoc """
  Function call without opaqueness type mismatch.

  ## Example

      defmodule OpaqueStruct do
        defstruct [:opaque]

        @opaque t :: %OpaqueStruct{}
      end

      defmodule Example do
        @spec error(OpaqueStruct.t()) :: :error
        def error(struct = %OpaqueStruct{}) do
          do_error(struct)
        end

        @spec do_error(OpaqueStruct.t()) :: :error
        defp do_error(_) do
          :error
        end
      end
  """

  @behaviour Dialyxir.Warning

  @impl Dialyxir.Warning
  @spec warning() :: :call_without_opaque
  def warning(), do: :call_without_opaque

  @impl Dialyxir.Warning
  @spec format_short([String.t()]) :: String.t()
  def format_short([_module, function | _]) do
    "Type mismatch in call without opaque term in #{function}."
  end

  @impl Dialyxir.Warning
  @spec format_long([String.t()]) :: String.t()
  def format_long([module, function, args, expected_triples]) do
    expected = form_expected_without_opaque(expected_triples)
    pretty_module = Erlex.pretty_print(module)
    pretty_args = Erlex.pretty_print_args(args)

    """
    Function call without opaqueness type mismatch.

    Call does not have expected #{expected}.

    #{pretty_module}.#{function}#{pretty_args}
    """
  end

  # We know which positions N are to blame;
  # the list of triples will never be empty.
  defp form_expected_without_opaque([{position, type, type_string}]) do
    pretty_type = Erlex.pretty_print_type(type_string)
    form_position_string = Dialyxir.WarningHelpers.form_position_string([position])

    if :erl_types.t_is_opaque(type) do
      "opaque term of type #{pretty_type} in the #{form_position_string} position"
    else
      "term of type #{pretty_type} (with opaque subterms) in the #{form_position_string} position"
    end
  end

  # TODO: can do much better here
  defp form_expected_without_opaque(expected_triples) do
    {arg_positions, _typess, _type_strings} = :lists.unzip3(expected_triples)
    form_position_string = Dialyxir.WarningHelpers.form_position_string(arg_positions)
    "opaque terms in the #{form_position_string} position"
  end

  @impl Dialyxir.Warning
  @spec explain() :: String.t()
  def explain() do
    @moduledoc
  end
end
