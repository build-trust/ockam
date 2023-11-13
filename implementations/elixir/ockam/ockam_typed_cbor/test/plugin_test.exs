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
      field(:name, Test.Name, minicbor: [schema: Test.Name, key: 1])

      field(:addresses, list(Test.Address.t()),
        minicbor: [key: 2, schema: {:list, Test.Address.minicbor_schema()}]
      )

      field(:age, integer() | nil, minicbor: [key: 3])
      field(:gender, :male | :female, minicbor: [key: 4, schema: {:enum, [male: 0, female: 1]}])
      field(:like_shoes, :boolean, minicbor: [key: 5])
      field(:nicknames, list(String.t()) | nil, minicbor: [key: 6])
    end
  end

  defmodule Test.Name do
    defstruct [:firstname, :lastname]

    # Encode it as a 2-element array of binaries
    def to_cbor_term(%Test.Name{firstname: f, lastname: l}) do
      {:ok, [%CBOR.Tag{tag: :bytes, value: f}, %CBOR.Tag{tag: :bytes, value: l}]}
    end

    def to_cbor_term(_), do: :error

    def from_cbor_term([%CBOR.Tag{tag: :bytes, value: f}, %CBOR.Tag{tag: :bytes, value: l}]) do
      {:ok, %Test.Name{firstname: f, lastname: l}}
    end

    def from_cbor_term(_), do: :error
  end

  defmodule Test.CreditCard do
    use TypedStruct

    typedstruct do
      plugin(Ockam.TypedCBOR.Plugin, encode_as: :list)

      field(:number, String.t(), minicbor: [key: 1])
      field(:exp_month, integer(), minicbor: [key: 2])
      field(:exp_year, integer(), minicbor: [key: 3])
      field(:cvv, integer(), minicbor: [key: 4])
    end
  end

  describe "plugin compilation" do
    test "errors with missing minicbor option" do
      assert_raise RuntimeError, fn ->
        Code.compile_string("""
        defmodule Test.Minicbor.Error do
          use TypedStruct

          typedstruct do
            plugin(Ockam.TypedCBOR.Plugin)
            field(:name, String.t())
          end
        end
        """)
      end
    end

    test "errors when encoding is list and keys are not sequential" do
      assert_raise RuntimeError, fn ->
        Code.compile_string("""
        defmodule Test.Keys.When.List do
          use TypedStruct

          typedstruct do
            plugin(Ockam.TypedCBOR.Plugin, encode_as: :list)
            field(:one, integer(), minicbor: [key: 1])
            field(:three, integer(), minicbor: [key: 3])
          end
        end
        """)
      end
    end

    test "issues warning when encoding is struct and keys are not sequential" do
      capture =
        ExUnit.CaptureIO.capture_io(:stderr, fn ->
          Code.compile_string("""
          defmodule Test.Keys.When.Struct do
            use TypedStruct

            typedstruct do
              plugin(Ockam.TypedCBOR.Plugin)
              field(:one, integer(), minicbor: [key: 1])
              field(:three, integer(), minicbor: [key: 3])
            end
          end
          """)
        end)

      assert String.starts_with?(capture, "\e[33mwarning:")
    end
  end

  test "encode-decode" do
    name = %Test.Name{firstname: "john", lastname: "smith"}
    p = %Test.Person{name: name, age: 23, gender: :male, addresses: [], like_shoes: false}
    {:ok, data} = Test.Person.encode(p)
    {:ok, ^p, ""} = Test.Person.decode(data)

    p = %Test.Person{p | age: nil}
    {:ok, data} = Test.Person.encode(p)
    assert {:ok, ^p, ""} = Test.Person.decode(data)

    p = %Test.Person{
      p
      | addresses: [
          %Test.Address{city: "ny", state: "ny", street: "5th av"},
          %Test.Address{city: "ny", state: "ny", street: "5th av"}
        ]
    }

    {:ok, data} = Test.Person.encode(p)
    assert {:ok, ^p, ""} = Test.Person.decode(data)

    p = %Test.Person{p | nicknames: ["aa", "bb"]}
    {:ok, data} = Test.Person.encode(p)
    assert {:ok, ^p, ""} = Test.Person.decode(data)

    {:ok, data} = Test.Person.encode_list([p, p])
    assert {:ok, [^p, ^p], ""} = Test.Person.decode_list(data)

    cc = %Test.CreditCard{number: "1234-1111-2222-3333", exp_month: 12, exp_year: 2030, cvv: 123}
    {:ok, data} = Test.CreditCard.encode(cc)
    assert {:ok, ^cc, ""} = Test.CreditCard.decode(data)
  end

  test "encode errors" do
    p = %Test.Person{}
    # Missing required fields
    {:error, _} = Test.Person.encode(p)

    # Incorrect type
    p = %Test.Person{name: ["Test"], age: 23, gender: :male, addresses: [], like_shoes: false}

    assert {:error, "type mismatch, expected schema Ockam.TypedCBOR.Plugin.Test.Test.Name"} =
             Test.Person.encode(p)
  end

  test "decode errors" do
    # Some fields are missing
    {:error, _} = Test.Person.decode(CBOR.encode(%{}))

    # Incorrect type for field 1 (name)
    assert {:error, "type mismatch, expected schema Ockam.TypedCBOR.Plugin.Test.Test.Name"} =
             Test.Person.decode(CBOR.encode(%{1 => 22, 2 => [], 4 => 0, 5 => true}))

    # Garbage data
    assert {:error, _} = Test.Person.decode(<<200>>)
  end
end
