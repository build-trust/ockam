defmodule Dialyxir.WarningHelpers do
  @spec ordinal(non_neg_integer) :: String.t()
  def ordinal(1), do: "1st"
  def ordinal(2), do: "2nd"
  def ordinal(3), do: "3rd"
  def ordinal(n) when is_integer(n), do: "#{n}th"

  def call_or_apply_to_string(
        arg_positions,
        :only_sig,
        signature_args,
        _signature_return,
        {_overloaded?, _contract}
      ) do
    pretty_signature_args = Erlex.pretty_print_args(signature_args)

    if Enum.empty?(arg_positions) do
      # We do not know which argument(s) caused the failure
      """
      will never return since the success typing arguments are
      #{pretty_signature_args}
      """
    else
      positions = form_position_string(arg_positions)

      """
      will never return since the #{positions} arguments differ
      from the success typing arguments:

      #{pretty_signature_args}
      """
    end
  end

  def call_or_apply_to_string(
        arg_positions,
        :only_contract,
        _signature_args,
        _signature_return,
        {overloaded?, contract}
      ) do
    pretty_contract = Erlex.pretty_print_contract(contract)

    if Enum.empty?(arg_positions) || overloaded? do
      # We do not know which arguments caused the failure
      """
      breaks the contract
      #{pretty_contract}
      """
    else
      position_string = form_position_string(arg_positions)

      """
      breaks the contract
      #{pretty_contract}

      in #{position_string} argument
      """
    end
  end

  def call_or_apply_to_string(
        _arg_positions,
        :both,
        signature_args,
        signature_return,
        {_overloaded?, contract}
      ) do
    pretty_contract = Erlex.pretty_print_contract(contract)

    pretty_print_signature =
      Erlex.pretty_print_contract("#{signature_args} -> #{signature_return}")

    """
    will never return since the success typing is:
    #{pretty_print_signature}

    and the contract is
    #{pretty_contract}
    """
  end

  def form_position_string(arg_positions) do
    Enum.map_join(arg_positions, " and ", &ordinal/1)
  end
end
