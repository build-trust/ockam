defmodule Ockam.ABAC.AttributeRule.Formatter.Tests do
  use ExUnit.Case

  alias Ockam.ABAC.AttributeRule

  describe "attribute rule formatting/parsing" do
    test "format/parse simple rules" do
      rules = [
        {true, "true"},
        {false, "false"},
        {{:eq, {:subject, "foo"}, "bar"}, "(= subject.foo \"bar\")"},
        {{:neq, {:subject, "foo"}, "bar"}, "(!= subject.foo \"bar\")"},
        {{:gt, {:subject, "foo"}, {:action, "bar"}}, "(> subject.foo action.bar)"},
        {{:lt, {:subject, "foo"}, {:action, "bar"}}, "(< subject.foo action.bar)"},
        {{:member, {:subject, "foo"}, [1]}, "(member? subject.foo [1])"},
        {{:member, {:subject, "foo"}, {:action, "bar"}}, "(member? subject.foo action.bar)"},
        {{:exists, {:subject, "foo"}}, "(exists? subject.foo)"},
        {{:member, "foo", {:action, "bar"}}, "(member? \"foo\" action.bar)"},
        {{:not, false}, "(not false)"},
        {{:if, {:eq, {:subject, "foo"}, "bar"}, false, true},
         "(if (= subject.foo \"bar\") false true)"},
        {{:and, [true, false]}, "(and true false)"},
        {{:or, [{:eq, {:subject, "foo"}, "bar"}, false]}, "(or (= subject.foo \"bar\") false)"}
      ]

      Enum.each(rules, fn {rule, string} ->
        attribute_rule = %AttributeRule{rule: rule}
        assert ^string = AttributeRule.format(attribute_rule)
        assert {:ok, ^attribute_rule} = AttributeRule.parse(string)
      end)
    end
  end
end
