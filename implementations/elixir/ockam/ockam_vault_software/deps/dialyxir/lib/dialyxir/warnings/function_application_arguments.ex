defmodule Dialyxir.Warnings.FunctionApplicationArguments do
  @behaviour Dialyxir.Warning

  @impl Dialyxir.Warning
  @spec warning() :: :fun_app_args
  def warning(), do: :fun_app_args

  @impl Dialyxir.Warning
  @spec format_short([String.t()]) :: String.t()
  def format_short([args, _type]) do
    pretty_args = Erlex.pretty_print_args(args)
    "Function application with #{pretty_args} will fail."
  end

  # OTP 22+ format
  def format_short([_arg_positions, args, type]) do
    format_short([args, type])
  end

  @impl Dialyxir.Warning
  @spec format_long([String.t()]) :: String.t()
  def format_long([args, type]) do
    pretty_args = Erlex.pretty_print_args(args)
    pretty_type = Erlex.pretty_print(type)

    "Function application with arguments #{pretty_args} will fail, " <>
      "because the function has type #{pretty_type}."
  end

  # OTP 22+ format
  def format_long([arg_positions, args, type]) do
    pretty_arg_positions = form_positions(arg_positions)
    pretty_args = Erlex.pretty_print_args(args)
    pretty_type = Erlex.pretty_print(type)

    "Function application with arguments #{pretty_args} will fail, " <>
      "because the function has type #{pretty_type}, " <>
      "which differs in #{pretty_arg_positions}."
  end

  defp form_positions(arg_positions = [_]) do
    form_position_string = Dialyxir.WarningHelpers.form_position_string(arg_positions)
    "the #{form_position_string} argument"
  end

  defp form_positions(arg_positions) do
    form_position_string = Dialyxir.WarningHelpers.form_position_string(arg_positions)
    "the #{form_position_string} arguments"
  end

  @impl Dialyxir.Warning
  @spec explain() :: String.t()
  def explain() do
    Dialyxir.Warning.default_explain()
  end
end
