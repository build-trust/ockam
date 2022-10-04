defmodule Ockam.ABAC.AttributeRule.Formatter do
  @moduledoc """
  Formatter module for Ockam.ABAC.AttributeRule
  Converts rule to string
  """

  alias Ockam.ABAC.AttributeRule
  require AttributeRule

  def format(bool) when is_boolean(bool) do
    to_string(bool)
  end

  def format({op, one, other}) do
    "(#{format_op(op)} #{format_arg(one)} #{format_arg(other)})"
  end

  def format({:exists, key}) do
    "(exists? #{format_arg(key)})"
  end

  def format({comb, rules}) when comb == :and or (comb == :or and is_list(rules)) do
    formatted_rules = Enum.map_join(rules, " ", fn rule -> format(rule) end)
    "(#{format_comb(comb)} #{formatted_rules})"
  end

  def format({:not, rule}) do
    "(not #{format(rule)})"
  end

  def format({:if, condition, true_rule, false_rule}) do
    "(if #{format(condition)} #{format(true_rule)} #{format(false_rule)})"
  end

  def format_comb(comb) do
    to_string(comb)
  end

  def format_op(:eq), do: "="
  def format_op(:neq), do: "!="
  def format_op(:gt), do: ">"
  def format_op(:lt), do: "<"
  def format_op(:member), do: "member?"

  def format_arg({type, name} = key) when AttributeRule.is_key(key) do
    "#{type}.#{name}"
  end

  def format_arg(number) when is_number(number) do
    to_string(number)
  end

  def format_arg(str) when is_binary(str) do
    "\"#{str}\""
  end

  def format_arg(bool) when is_boolean(bool) do
    to_string(bool)
  end

  def format_arg(list) when is_list(list) do
    formatted_list = Enum.map_join(list, " ", fn el -> format_arg(el) end)
    "[#{formatted_list}]"
  end

  def format_arg(other) do
    raise "Cannot format argument: #{inspect(other)}"
  end
end
