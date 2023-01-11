defmodule Ockam.TypedCBOR.Plugin.Test do
  use ExUnit.Case

  defmodule Test.Address do
    use TypedStruct

    typedstruct do
      plugin(Ockam.TypedCBOR.Plugin)
      field(:city, String.t(), minicbor: [key: 1])
      field(:state, String.t(), minicbor: [key: 2])
      field(:street, String.t() | nil, minicbor: [key: 3])
    end
  end

  defmodule Test.Person do
    use TypedStruct

    typedstruct do
      plugin(Ockam.TypedCBOR.Plugin)
      field(:name, String.t(), minicbor: [key: 1])

      field(:addresses, list(Test.Address.t()),
        minicbor: [key: 2, schema: {:list, Test.Address.minicbor_schema()}]
      )

      field(:age, integer() | nil, minicbor: [key: 3])
      field(:gender, :male | :female, minicbor: [key: 4, schema: {:enum, [male: 0, female: 1]}])
      field(:like_shoes, :boolean, minicbor: [key: 5])
      field(:nicknames, list(String.t()) | nil, minicbor: [key: 6])
    end
  end

  test "encode-decode" do
    p = %Test.Person{name: "Test", age: 23, gender: :male, addresses: [], like_shoes: false}
    {:ok, data} = Test.Person.encode(p)
    {:ok, ^p, ""} = Test.Person.decode(data)

    p = %Test.Person{p | age: nil}
    {:ok, data} = Test.Person.encode(p)
    {:ok, ^p, ""} = Test.Person.decode(data)

    p = %Test.Person{
      p
      | addresses: [
          %Test.Address{city: "ny", state: "ny", street: "5th av"},
          %Test.Address{city: "ny", state: "ny", street: "5th av"}
        ]
    }

    {:ok, data} = Test.Person.encode(p)
    {:ok, ^p, ""} = Test.Person.decode(data)

    p = %Test.Person{p | nicknames: ["aa", "bb"]}
    {:ok, data} = Test.Person.encode(p)
    {:ok, ^p, ""} = Test.Person.decode(data)

    {:ok, data} = Test.Person.encode_list([p, p])
    {:ok, [^p, ^p], ""} = Test.Person.decode_list(data)
  end

  test "encode errors" do
    p = %Test.Person{}
    # Missing required fields
    {:error, _} = Test.Person.encode(p)

    # Incorrect type
    p = %Test.Person{name: ["Test"], age: 23, gender: :male, addresses: [], like_shoes: false}
    {:error, "type mismatch, expected schema :string, value: [\"Test\"]"} = Test.Person.encode(p)
  end

  test "decode errors" do
    # Some fields are missing
    {:error, _} = Test.Person.decode(CBOR.encode(%{}))

    # Incorrect type for field 1 (name)
    {:error, "type mismatch, expected schema :string"} =
      Test.Person.decode(CBOR.encode(%{1 => 22, 2 => [], 4 => 0, 5 => true}))

    # Garbage data
    {:error, _} = Test.Person.decode(<<200>>)
  end
end
