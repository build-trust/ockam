defmodule Ockam.ABAC.AttributeRule.Tests do
  use ExUnit.Case

  alias Ockam.ABAC.ActionId
  alias Ockam.ABAC.AttributeRule
  alias Ockam.ABAC.Request

  describe "single attribute rule" do
    test "eq rule" do
      request_matching = %Request{
        action_id: ActionId.new("", ""),
        resource_attributes: %{},
        subject_attributes: %{"foo" => "bar"},
        action_attributes: %{}
      }

      request_not_matching = %Request{
        action_id: ActionId.new("", ""),
        resource_attributes: %{},
        subject_attributes: %{"foo" => "baz"},
        action_attributes: %{}
      }

      {:ok, rule} = AttributeRule.parse("(= subject.foo \"bar\")")
      assert AttributeRule.match_rule?(rule, request_matching)
      refute AttributeRule.match_rule?(rule, request_not_matching)
    end

    test "neq rule" do
      request_matching = %Request{
        action_id: ActionId.new("", ""),
        resource_attributes: %{},
        subject_attributes: %{"foo" => "bar"},
        action_attributes: %{}
      }

      request_not_matching = %Request{
        action_id: ActionId.new("", ""),
        resource_attributes: %{},
        subject_attributes: %{"foo" => "baz"},
        action_attributes: %{}
      }

      {:ok, rule} = AttributeRule.parse("(!= subject.foo \"baz\")")
      assert AttributeRule.match_rule?(rule, request_matching)
      refute AttributeRule.match_rule?(rule, request_not_matching)
    end

    test "gt rule" do
      request_matching = %Request{
        action_id: ActionId.new("", ""),
        resource_attributes: %{},
        subject_attributes: %{"foo" => 3},
        action_attributes: %{}
      }

      request_not_matching = %Request{
        action_id: ActionId.new("", ""),
        resource_attributes: %{},
        subject_attributes: %{"foo" => 1},
        action_attributes: %{}
      }

      {:ok, rule} = AttributeRule.parse("(> subject.foo 2)")
      assert AttributeRule.match_rule?(rule, request_matching)
      refute AttributeRule.match_rule?(rule, request_not_matching)
    end

    test "lt rule" do
      request_matching = %Request{
        action_id: ActionId.new("", ""),
        resource_attributes: %{},
        subject_attributes: %{"foo" => 1},
        action_attributes: %{}
      }

      request_not_matching = %Request{
        action_id: ActionId.new("", ""),
        resource_attributes: %{},
        subject_attributes: %{"foo" => 3},
        action_attributes: %{}
      }

      {:ok, rule} = AttributeRule.parse("(< subject.foo 2)")
      assert AttributeRule.match_rule?(rule, request_matching)
      refute AttributeRule.match_rule?(rule, request_not_matching)
    end

    test "member rule" do
      request_matching1 = %Request{
        action_id: ActionId.new("", ""),
        resource_attributes: %{},
        subject_attributes: %{"foo" => "bar"},
        action_attributes: %{}
      }

      request_matching2 = %Request{
        action_id: ActionId.new("", ""),
        resource_attributes: %{},
        subject_attributes: %{"foo" => "baf"},
        action_attributes: %{}
      }

      request_not_matching = %Request{
        action_id: ActionId.new("", ""),
        resource_attributes: %{},
        subject_attributes: %{"foo" => "baz"},
        action_attributes: %{}
      }

      {:ok, rule} =
        AttributeRule.parse("""
          (member? subject.foo ["bar" "baf"])
        """)

      assert AttributeRule.match_rule?(rule, request_matching1)
      assert AttributeRule.match_rule?(rule, request_matching2)
      refute AttributeRule.match_rule?(rule, request_not_matching)
    end
  end

  describe "multi attribute rule" do
    test "eq rule" do
      request_matching = %Request{
        action_id: ActionId.new("", ""),
        resource_attributes: %{},
        subject_attributes: %{"foo1" => "bar"},
        action_attributes: %{"foo2" => "bar"}
      }

      request_not_matching = %Request{
        action_id: ActionId.new("", ""),
        resource_attributes: %{},
        subject_attributes: %{"foo1" => "bar"},
        action_attributes: %{"foo2" => "not_bar"}
      }

      {:ok, rule} = AttributeRule.parse("(= subject.foo1 action.foo2)")
      assert AttributeRule.match_rule?(rule, request_matching)
      refute AttributeRule.match_rule?(rule, request_not_matching)
    end

    test "neq rule" do
      request_matching = %Request{
        action_id: ActionId.new("", ""),
        resource_attributes: %{},
        subject_attributes: %{"foo1" => "bar"},
        action_attributes: %{"foo2" => "not_bar"}
      }

      request_not_matching = %Request{
        action_id: ActionId.new("", ""),
        resource_attributes: %{},
        subject_attributes: %{"foo1" => "bar"},
        action_attributes: %{"foo2" => "bar"}
      }

      {:ok, rule} = AttributeRule.parse("(!= subject.foo1 action.foo2)")
      assert AttributeRule.match_rule?(rule, request_matching)
      refute AttributeRule.match_rule?(rule, request_not_matching)
    end

    test "gt rule" do
      request_matching = %Request{
        action_id: ActionId.new("", ""),
        resource_attributes: %{},
        subject_attributes: %{"foo1" => 2},
        action_attributes: %{"foo2" => 1}
      }

      request_not_matching = %Request{
        action_id: ActionId.new("", ""),
        resource_attributes: %{},
        subject_attributes: %{"foo1" => 2},
        action_attributes: %{"foo2" => 3}
      }

      {:ok, rule} = AttributeRule.parse("(> subject.foo1 action.foo2)")
      assert AttributeRule.match_rule?(rule, request_matching)
      refute AttributeRule.match_rule?(rule, request_not_matching)
    end

    test "lt rule" do
      request_matching = %Request{
        action_id: ActionId.new("", ""),
        resource_attributes: %{},
        subject_attributes: %{"foo1" => 2},
        action_attributes: %{"foo2" => 3}
      }

      request_not_matching = %Request{
        action_id: ActionId.new("", ""),
        resource_attributes: %{},
        subject_attributes: %{"foo1" => 2},
        action_attributes: %{"foo2" => 1}
      }

      {:ok, rule} = AttributeRule.parse("(< subject.foo1 action.foo2)")
      assert AttributeRule.match_rule?(rule, request_matching)
      refute AttributeRule.match_rule?(rule, request_not_matching)
    end

    test "member rule" do
      request_matching1 = %Request{
        action_id: ActionId.new("", ""),
        resource_attributes: %{},
        subject_attributes: %{"foo" => "bar"},
        action_attributes: %{"foo_list" => ["bar", "baf"]}
      }

      request_matching2 = %Request{
        action_id: ActionId.new("", ""),
        resource_attributes: %{},
        subject_attributes: %{"foo" => "baf"},
        action_attributes: %{"foo_list" => ["bar", "baf"]}
      }

      request_not_matching = %Request{
        action_id: ActionId.new("", ""),
        resource_attributes: %{},
        subject_attributes: %{"foo" => "baz"},
        action_attributes: %{}
      }

      {:ok, rule} = AttributeRule.parse("(member? subject.foo action.foo_list)")

      assert AttributeRule.match_rule?(rule, request_matching1)
      assert AttributeRule.match_rule?(rule, request_matching2)
      refute AttributeRule.match_rule?(rule, request_not_matching)
    end
  end

  test "exists rule" do
    request_matching = %Request{
      action_id: ActionId.new("", ""),
      resource_attributes: %{},
      subject_attributes: %{"foo" => "bar"},
      action_attributes: %{}
    }

    request_not_matching = %Request{
      action_id: ActionId.new("", ""),
      resource_attributes: %{},
      subject_attributes: %{},
      action_attributes: %{"foo" => "bar"}
    }

    {:ok, rule} = AttributeRule.parse("(exists? subject.foo)")

    assert AttributeRule.match_rule?(rule, request_matching)
    refute AttributeRule.match_rule?(rule, request_not_matching)
  end

  describe "logic rules" do
    test "simple rules" do
      empty_request = %Request{
        action_id: ActionId.new("", ""),
        subject_attributes: %{},
        action_attributes: %{},
        resource_attributes: %{}
      }

      {:ok, true_rule} = AttributeRule.parse("true")
      assert AttributeRule.match_rule?(true_rule, empty_request)

      {:ok, false_rule} = AttributeRule.parse("false")
      refute AttributeRule.match_rule?(false_rule, empty_request)

      {:ok, and_rule} = AttributeRule.parse("(and true false)")
      refute AttributeRule.match_rule?(and_rule, empty_request)

      {:ok, or_rule} = AttributeRule.parse("(or true false)")
      assert AttributeRule.match_rule?(or_rule, empty_request)

      {:ok, not_rule} = AttributeRule.parse("(not false)")
      assert AttributeRule.match_rule?(not_rule, empty_request)
    end

    test "combination rules" do
      request_matching = %Request{
        action_id: ActionId.new("", ""),
        subject_attributes: %{"name" => "Ivan"},
        action_attributes: %{"method" => "get"},
        resource_attributes: %{"people" => ["Ivan", "Marya"]}
      }

      request_not_matching1 = %Request{
        action_id: ActionId.new("", ""),
        subject_attributes: %{"name" => "Ivan"},
        action_attributes: %{"method" => "post"},
        resource_attributes: %{"people" => ["Ivan", "Marya"]}
      }

      request_not_matching2 = %Request{
        action_id: ActionId.new("", ""),
        subject_attributes: %{"name" => "Sergey"},
        action_attributes: %{"method" => "get"},
        resource_attributes: %{"people" => ["Ivan", "Marya"]}
      }

      {:ok, rule} = AttributeRule.parse("(and
          (= action.method \"get\")
          (member? subject.name resource.people))")

      assert AttributeRule.match_rule?(rule, request_matching)
      refute AttributeRule.match_rule?(rule, request_not_matching1)
      refute AttributeRule.match_rule?(rule, request_not_matching2)
    end
  end
end
