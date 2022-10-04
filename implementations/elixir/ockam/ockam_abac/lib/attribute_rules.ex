defmodule Ockam.ABAC.AttributeRule do
  @moduledoc """
  Attribute rule matching AST for Ockam.ABAC
  """

  alias Ockam.ABAC.AttributeRule.Formatter

  alias Ockam.ABAC.Request

  @type attribute_source() :: :resource | :action | :subject
  @type key() :: {source :: attribute_source(), name :: binary()}
  @type value() :: binary() | number() | boolean()

  ## TODO: Maybe we should use the same operations format
  ## for internal representation as we do in the s-expr format ('=', '!=' etc)
  @type rule() ::
          true
          | false
          | {:eq, key(), value()}
          | {:eq, key(), key()}
          | {:member, key(), [value()]}
          | {:member, key(), key()}
          | {:exists, key()}
          | {:lt, key(), value()}
          | {:lt, key(), key()}
          | {:gt, key(), value()}
          | {:gt, key(), key()}
          | {:not, rule()}
          | {:and, [rule()]}
          | {:or, [rule()]}
          | {:if, rule(), rule(), rule()}

  defstruct [:rule]

  defguard is_key(key)
           when is_tuple(key) and tuple_size(key) == 2 and
                  (elem(key, 0) == :resource or elem(key, 0) == :action or
                     elem(key, 0) == :subject) and
                  is_binary(elem(key, 1))

  ## TODO: support more value types for gt/lt
  defguard is_value(value) when is_binary(value) or is_boolean(value) or is_number(value)

  defguard is_filter(filter)
           when filter == :eq or filter == :lt or filter == :gt or filter == :neq

  def parse(string) do
    ## attribute_rule_grammar is built dynamically by a mix task
    ## `apply/3` is used to silence a warning about missing module
    # credo:disable-for-next-line
    case apply(:attribute_rule_grammar, :parse, [string]) do
      {:fail, _reason} ->
        {:error, {:cannot_parse_rule, string}}

      parsed_rule ->
        new(parsed_rule)
    end
  end

  def format(%__MODULE__{rule: rule}) do
    Formatter.format(rule)
  end

  @doc false
  def new(rule_def) do
    case validate(rule_def) do
      :ok ->
        {:ok, %__MODULE__{rule: rule_def}}

      {:error, reason} ->
        {:error, reason}
    end
  end

  def match_rule?(%__MODULE__{rule: rule}, %Request{} = request) do
    do_match_rule?(rule, request)
  end

  defp do_match_rule?({op, key, value}, request)
       when is_filter(op) and is_key(key) and is_value(value) do
    case fetch_attribute(request, key) do
      {:ok, attribute_value} -> compare(op, attribute_value, value)
      :error -> false
    end
  end

  defp do_match_rule?({op, key1, key2}, request)
       when is_filter(op) and is_key(key1) and is_key(key2) do
    with {:ok, val1} <- fetch_attribute(request, key1),
         {:ok, val2} <- fetch_attribute(request, key2) do
      compare(op, val1, val2)
    else
      _other ->
        ## TODO: improve match failure reporting
        false
    end
  end

  defp do_match_rule?({:member, element, list}, request) do
    with {:ok, element} <- member_element(element, request),
         {:ok, list} <- member_list(list, request) do
      Enum.member?(list, element)
    else
      _other ->
        ## TODO: improve match failure reporting
        false
    end
  end

  defp do_match_rule?({:exists, key}, request) when is_key(key) do
    case fetch_attribute(request, key) do
      {:ok, _val} -> true
      :error -> false
    end
  end

  defp do_match_rule?({:not, rule}, request) do
    not do_match_rule?(rule, request)
  end

  defp do_match_rule?({:and, rules_list}, request) do
    Enum.all?(rules_list, fn rule -> do_match_rule?(rule, request) end)
  end

  defp do_match_rule?({:or, rules_list}, request) do
    Enum.any?(rules_list, fn rule -> do_match_rule?(rule, request) end)
  end

  defp do_match_rule?({:if, condition, true_rule, false_rule}, request) do
    case do_match_rule?(condition, request) do
      true ->
        do_match_rule?(true_rule, request)

      false ->
        do_match_rule?(false_rule, request)
    end
  end

  defp do_match_rule?(true, _request) do
    true
  end

  defp do_match_rule?(false, _request) do
    false
  end

  defp do_match_rule?(_rule, _request) do
    false
  end

  defp compare(:eq, val1, val2), do: val1 == val2
  defp compare(:neq, val1, val2), do: val1 != val2
  defp compare(:gt, val1, val2), do: val1 > val2
  defp compare(:lt, val1, val2), do: val1 < val2

  defp member_element(key, request) when is_key(key) do
    fetch_attribute(request, key)
  end

  defp member_element(value, _request) when is_value(value) do
    {:ok, value}
  end

  defp member_list(key, request) when is_key(key) do
    case fetch_attribute(request, key) do
      {:ok, list} when is_list(list) -> {:ok, list}
      _other -> :error
    end
  end

  defp member_list(list, _request) when is_list(list) do
    {:ok, list}
  end

  def fetch_attribute(%Request{} = request, {type, name}) do
    request
    |> Map.get(atrtibute_field(type), %{})
    |> Map.fetch(name)
  end

  defp atrtibute_field(:resource), do: :resource_attributes
  defp atrtibute_field(:action), do: :action_attributes
  defp atrtibute_field(:subject), do: :subject_attributes

  ## Credo is complaining about this function complexity
  # credo:disable-for-next-line
  defp validate(rule) do
    case rule do
      bool when is_boolean(bool) ->
        :ok

      {filter, key, key_or_value}
      when is_filter(filter) and is_key(key) and (is_key(key_or_value) or is_value(key_or_value)) ->
        :ok

      {:exists, key} when is_key(key) ->
        :ok

      {:member, key_or_value, key}
      when is_key(key) and (is_key(key_or_value) or is_value(key_or_value)) ->
        :ok

      {:member, key_or_value, list} = rule
      when is_list(list) and (is_key(key_or_value) or is_value(key_or_value)) ->
        validate_list(list, rule)

      {comb, [_rule1, _rule2 | _other] = rules} when comb == :and or comb == :or ->
        validate_inner_rules(rules)

      {:not, rule} ->
        validate(rule)

      {:if, condition, true_rule, false_rule} ->
        validate_inner_rules([condition, true_rule, false_rule])

      invalid ->
        {:error, {:invalid_rule, invalid}}
    end
  end

  defp validate_list(list, rule) do
    valid_elements = Enum.all?(list, fn el -> is_value(el) end)

    case valid_elements do
      true ->
        :ok

      false ->
        {:error, {:invalid_list_elements, rule}}
    end
  end

  defp validate_inner_rules(rules) do
    errors =
      rules
      |> Enum.map(fn rule -> validate(rule) end)
      |> Enum.filter(fn
        :ok -> false
        _error -> true
      end)

    case errors do
      [] -> :ok
      errors -> {:error, {:internal_rules_invalid, errors}}
    end
  end
end
