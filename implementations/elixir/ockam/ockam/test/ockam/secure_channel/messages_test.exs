defmodule Ockam.SecureChannel.Messages.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.SecureChannel.Messages
  alias Ockam.Address
  alias Ockam.SecureChannel.Messages
  alias Ockam.SecureChannel.Messages.Payload
  alias Ockam.SecureChannel.Messages.PayloadPart
  alias Ockam.SecureChannel.Messages.PayloadParts

  describe "Ockam.SecureChannel.Messages.RefreshCredentials" do
    test "refresh credential can be parsed" do
      # sample value encoded from rust
      hex_msg =
        "8201818281825837830101583285f682008158208bd01513a019c95d96553015b1b0a3014e7bd67c7f3c8e6223e839111041f8d6f41a659c99fd1a78689cfd820081584067118992d037593809f8b217641aed54dc983f2847f95b2068207c3647beb73a60b0e14012c14bfee487c880f0f74d9f8a7d7abd3ef333f2f90f097ef366900c81828258e183010358dc855820904192e6e480915e0d1256d3abf9c3c9c67bc2b4c41ffca77550d8a3f12fbb1e5820904192e6e480915e0d1256d3abf9c3c9c67bc2b4c41ffca77550d8a3f12fbb1e8201a44b6f636b616d2d72656c6179412a4a6f636b616d2d726f6c6548656e726f6c6c65724a70726f6a6563745f6964582437363764343631632d393835312d346330622d626663362d3664333333346235626363325074727573745f636f6e746578745f6964582437363764343631632d393835312d346330622d626663362d3664333333346235626363321a659ca3cf1a659ca3ed8200815840cf27a0a3052c53b1f27bf7776ca95c7dcdd7a0f493cc5b1e30f9d35712bb8fde16f69065aa9f70b3689a8f1b7b05fc13e6e807efa719031d1a415d25b61c360b82587c8301025877855820b1fea1775d75079abdb1e78b96921fa9ec340bc2b5aa70f37e65342d859cf5505820b1fea1775d75079abdb1e78b96921fa9ec340bc2b5aa70f37e65342d859cf5508201818200815820389852b0f4fee7b6b962442a924d1672c56b8813fb846a50da80db7e1bbe41591a659c9a2f1a6f029baf8200815840adf4a66e097de839d3539694a6c6a82a978ff009205df8bf7eb03b9990958b7c4e7037fd9d365cf220a2757ed60f0542f47965d8b9f354fd950841ce66304606"

      {:ok, b} = Base.decode16(hex_msg, case: :lower)
      {:ok, %Messages.RefreshCredentials{}} = Messages.decode(b)
    end
  end

  describe "Ockam.SecureChannel.Messages.Close" do
    test ":close can be parsed" do
      # sample value encoded from rust
      hex_msg = "820280"

      {:ok, b} = Base.decode16(hex_msg, case: :lower)
      {:ok, :close} = Messages.decode(b)
      {:ok, ^b} = Messages.encode(:close)
    end
  end

  describe "Ockam.SecureChannel.Messages.Payload" do
    test "A payload can be encoded then decoded" do
      expected = %Payload{
        onward_route: Address.parse_route!("1#onward_route"),
        return_route: Address.parse_route!("1#return_route"),
        payload: <<1, 2, 3>>
      }

      {:ok, encoded} = Messages.encode(expected)

      # sanity check and use this value on the Rust side in messages.rs
      as_hex = Base.encode16(encoded, case: :lower)

      assert as_hex ==
               "8200818381a20101028c186f186e1877186118721864185f1872186f18751874186581a20101028c18721865187418751872186e185f1872186f18751874186543010203"

      {:ok, actual} = Messages.decode(encoded)
      assert actual == expected
    end

    test "A payload can be decoded from rust" do
      hex_msg =
        "8200818381a20101028c186f186e1877186118721864185f1872186f18751874186581a20101028c18721865187418751872186e185f1872186f18751874186543010203"

      expected = %Payload{
        onward_route: Address.parse_route!("1#onward_route"),
        return_route: Address.parse_route!("1#return_route"),
        payload: <<1, 2, 3>>
      }

      {:ok, b} = Base.decode16(hex_msg, case: :lower)
      {:ok, actual} = Messages.decode(b)

      assert actual == expected
    end
  end

  describe "Ockam.SecureChannel.Messages.PayloadParts" do
    test "A payload part can be encoded then decoded" do
      expected = %PayloadPart{
        onward_route: Address.parse_route!("1#onward_route"),
        return_route: Address.parse_route!("1#return_route"),
        payload: <<1, 2, 3>>,
        current_part_number: 1,
        total_number_of_parts: 3,
        payload_uuid: "24922fc8-ea4c-4387-b069-e2b296e0de7d"
      }

      {:ok, encoded} = Messages.encode(expected)

      # sanity check and use this value on the Rust side in messages.rs
      as_hex = Base.encode16(encoded, case: :lower)

      assert as_hex ==
               "8203818681a20101028c186f186e1877186118721864185f1872186f18751874186581a20101028c18721865187418751872186e185f1872186f187518741865430102030103782432343932326663382d656134632d343338372d623036392d653262323936653064653764"

      {:ok, actual} = Messages.decode(encoded)
      assert actual == expected
    end

    test "A payload part can be decoded from rust" do
      hex_msg =
        "8203818681a20101028c186f186e1877186118721864185f1872186f18751874186581a20101028c18721865187418751872186e185f1872186f187518741865430102030103782432343932326663382d656134632d343338372d623036392d653262323936653064653764"

      expected = %PayloadPart{
        onward_route: Address.parse_route!("1#onward_route"),
        return_route: Address.parse_route!("1#return_route"),
        payload: <<1, 2, 3>>,
        current_part_number: 1,
        total_number_of_parts: 3,
        payload_uuid: "24922fc8-ea4c-4387-b069-e2b296e0de7d"
      }

      {:ok, b} = Base.decode16(hex_msg, case: :lower)
      {:ok, actual} = Messages.decode(b)

      assert actual == expected
    end

    test "A payload part can be validated" do
      parts = %PayloadParts{
        uuid: UUID.uuid4(),
        parts: %{1 => <<1, 2, 3>>},
        onward_route: Address.parse_route!("1#onward_route"),
        return_route: Address.parse_route!("1#return_route"),
        expected_total_number_of_parts: 3,
        last_update: DateTime.utc_now()
      }

      # this part is ok
      assert PayloadParts.is_valid_part(
               parts,
               2,
               3,
               Address.parse_route!("1#onward_route"),
               Address.parse_route!("1#return_route")
             )

      # this part has an incorrect part number
      assert !PayloadParts.is_valid_part(
               parts,
               4,
               3,
               Address.parse_route!("1#onward_route"),
               Address.parse_route!("1#return_route")
             )

      # this part has an incorrect onward route
      assert !PayloadParts.is_valid_part(
               parts,
               2,
               3,
               Address.parse_route!("1#onward_route_x"),
               Address.parse_route!("1#return_route")
             )

      # this part has an incorrect return route
      assert !PayloadParts.is_valid_part(
               parts,
               2,
               3,
               Address.parse_route!("1#onward_route"),
               Address.parse_route!("1#return_route_x")
             )
    end

    test "When all the parts have been received, the payload is complete" do
      uuid = UUID.uuid4()

      # part 2 is the first received
      part_2 = %PayloadPart{
        onward_route: Address.parse_route!("1#onward_route"),
        return_route: Address.parse_route!("1#return_route"),
        payload: <<4, 5, 6>>,
        current_part_number: 2,
        total_number_of_parts: 3,
        payload_uuid: uuid
      }

      {:ok, parts} = PayloadParts.initialize(part_2, DateTime.utc_now())

      # add part 3
      part_3 = %PayloadPart{
        current_part_number: 3,
        total_number_of_parts: 3,
        onward_route: Address.parse_route!("1#onward_route"),
        return_route: Address.parse_route!("1#return_route"),
        payload: <<7, 8, 9>>,
        payload_uuid: uuid
      }

      {:ok, parts} = PayloadParts.update(parts, part_3, DateTime.utc_now())

      # the payload is not yet complete
      assert :error = PayloadParts.complete(parts)

      # add part 1
      part_1 = %PayloadPart{
        current_part_number: 1,
        total_number_of_parts: 3,
        onward_route: Address.parse_route!("1#onward_route"),
        return_route: Address.parse_route!("1#return_route"),
        payload: <<1, 2, 3>>,
        payload_uuid: uuid
      }

      {:ok, parts} = PayloadParts.update(parts, part_1, DateTime.utc_now())

      # the payload is now complete and assembled in the right order
      # even if the parts have been received in a different order
      assert {:ok, %Payload{payload: payload}} = PayloadParts.complete(parts)
      assert <<1, 2, 3, 4, 5, 6, 7, 8, 9>> = payload
    end

    test "The size of a multipart messages is limited" do
      uuid = UUID.uuid4()

      # first part of a message with 2001 parts
      part = %PayloadPart{
        onward_route: Address.parse_route!("1#onward_route"),
        return_route: Address.parse_route!("1#return_route"),
        payload: <<4, 5, 6>>,
        current_part_number: 1,
        total_number_of_parts: 2001,
        payload_uuid: uuid
      }

      result = PayloadParts.initialize(part, DateTime.utc_now())
      assert result = :error
    end
  end
end
