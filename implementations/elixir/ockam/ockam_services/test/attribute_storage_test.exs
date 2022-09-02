defmodule Test.Services.AttributeSrotageTest do
  use ExUnit.Case

  alias Ockam.Credential.AttributeStorageETS, as: AttributeStorage

  alias Ockam.Credential.AttributeSet
  alias Ockam.Credential.AttributeSet.Attributes

  test "stored attributes" do
    AttributeStorage.init()

    id = "foo"

    attribute_set = %AttributeSet{
      attributes: %Attributes{attributes: %{"project" => "123", "role" => "member"}},
      expiration: System.os_time(:second) + 100
    }

    AttributeStorage.put_attribute_set(id, attribute_set)

    assert attribute_set.attributes.attributes == AttributeStorage.get_attributes(id)
  end

  test "expired attributes" do
    AttributeStorage.init()

    id = "foo"

    attribute_set = %AttributeSet{
      attributes: %{"project" => "123", "role" => "member"},
      expiration: System.os_time(:second) - 100
    }

    AttributeStorage.put_attribute_set(id, attribute_set)

    assert %{} == AttributeStorage.get_attributes(id)
  end

  test "attribute storage missing" do
    id = "foo"

    attribute_set = %AttributeSet{
      attributes: %{"project" => "123", "role" => "member"},
      expiration: System.os_time(:second) + 100
    }

    AttributeStorage.put_attribute_set(id, attribute_set)

    assert %{} == AttributeStorage.get_attributes(id)
  end
end
