defmodule Ockam.SecureChannel.Messages do
  @moduledoc """
  Secure Channel protocol Messages
  """
  alias Ockam.Address
  alias Ockam.SecureChannel.Messages.RefreshCredentials
  alias Ockam.TypedCBOR

  require Logger

  defmodule AddressSchema do
    @moduledoc """
    Ockam Address, cbor encoding
    """
    use TypedStruct

    @address_schema {:struct,
                     %{
                       type: %{key: 1, schema: :integer, required: true},
                       value: %{key: 2, schema: :charlist, required: true}
                     }}
    def from_cbor_term(term) do
      addr = TypedCBOR.from_cbor_term(@address_schema, term)
      {:ok, Address.denormalize(addr)}
    end

    def to_cbor_term(addr) do
      {:ok, TypedCBOR.to_cbor_term(@address_schema, Address.normalize(addr))}
    end
  end

  defmodule Payload do
    @moduledoc """
    Secure channel message carrying user data
    """
    use TypedStruct

    typedstruct do
      plugin(TypedCBOR.Plugin, encode_as: :list)
      field(:onward_route, list(Address.t()), minicbor: [key: 0, schema: {:list, AddressSchema}])
      field(:return_route, list(Address.t()), minicbor: [key: 1, schema: {:list, AddressSchema}])
      field(:payload, binary(), minicbor: [key: 2])
    end
  end

  defmodule PayloadPart do
    @moduledoc """
    Part of a secure channel message payload
    """
    use TypedStruct

    typedstruct do
      plugin(TypedCBOR.Plugin, encode_as: :list)
      field(:onward_route, list(Address.t()), minicbor: [key: 0, schema: {:list, AddressSchema}])
      field(:return_route, list(Address.t()), minicbor: [key: 1, schema: {:list, AddressSchema}])
      field(:payload, binary(), minicbor: [key: 2])
      field(:current_part_number, integer(), minicbor: [key: 3])
      field(:total_number_of_parts, integer(), minicbor: [key: 4])
      field(:payload_uuid, String.t(), minicbor: [key: 5])
    end
  end

  defmodule PayloadParts do
    @moduledoc """
    List of payload parts received for a given payload UUID
    """

    use TypedStruct

    typedstruct do
      field(:uuid, String.t())
      field(:parts, map())
      field(:onward_route, Ockam.Address.route())
      field(:return_route, Ockam.Address.route())
      field(:expected_total_number_of_parts, integer())
      field(:last_update, DateTime.t())
    end

    # We only 2000 parts for a given multi-part message
    # Given the SecureChannel.@max_payload_part_size, this means that a message can have a size around 100Mb maximum
    @max_number_of_parts 2000

    # Start a new list of payload parts from the first received part
    def initialize(
          %PayloadPart{
            current_part_number: current_part_number,
            total_number_of_parts: total_number_of_parts,
            onward_route: onward_route,
            return_route: return_route,
            payload: payload,
            payload_uuid: payload_uuid
          },
          now
        ) do
      parts = %PayloadParts{
        uuid: payload_uuid,
        parts: %{},
        onward_route: onward_route,
        return_route: return_route,
        expected_total_number_of_parts: total_number_of_parts,
        last_update: now
      }

      if is_valid_part(
           parts,
           current_part_number,
           total_number_of_parts,
           onward_route,
           return_route
         ) do
        {:ok, %{parts | parts: %{current_part_number => payload}}}
      else
        {:error}
      end
    end

    # Check if the current part can be added to the other parts and return the updated PayloadParts
    def update(
          self,
          %PayloadPart{
            current_part_number: current_part_number,
            total_number_of_parts: total_number_of_parts,
            onward_route: onward_route,
            return_route: return_route,
            payload: payload
          },
          now
        ) do
      Logger.debug("updating current_part_number #{current_part_number}")

      if is_valid_part(
           self,
           current_part_number,
           total_number_of_parts,
           onward_route,
           return_route
         ) do
        {:ok,
         %{self | parts: Map.put(self.parts, current_part_number, payload), last_update: now}}
      else
        {:error}
      end
    end

    # Check if all the payload parts have been received and return the concatenated payload
    def complete(%__MODULE__{
          parts: parts,
          onward_route: onward_route,
          return_route: return_route,
          expected_total_number_of_parts: expected_total_number_of_parts
        }) do
      if Kernel.map_size(parts) == expected_total_number_of_parts do
        # get all the parts, sorted by key
        sorted_keys = parts |> Map.keys() |> Enum.sort()
        iodata = for key <- sorted_keys, do: Map.get(parts, key)
        payload = IO.iodata_to_binary(iodata)

        result = %Payload{
          onward_route: onward_route,
          return_route: return_route,
          payload: payload
        }

        {:ok, result}
      else
        :error
      end
    end

    # Return :ok if the current part can be added to the other parts
    def is_valid_part(
          self,
          current_part_number,
          total_number_of_parts,
          onward_route,
          return_route
        ) do
      cond do
        self.onward_route != onward_route ->
          Logger.error(
            "Incorrect onward route for part #{current_part_number}/#{total_number_of_parts} of message #{self.uuid}. Expected: #{inspect(self.onward_route)}, Got: #{inspect(onward_route)}"
          )

          false

        self.return_route != return_route ->
          Logger.error(
            "Incorrect return route for part #{current_part_number}/#{total_number_of_parts} of message #{self.uuid}. Expected: #{inspect(self.return_route)}, Got: #{inspect(return_route)}"
          )

          false

        self.expected_total_number_of_parts != total_number_of_parts ->
          Logger.error(
            "Incorrect total number of parts for part #{current_part_number}/#{total_number_of_parts} of message #{self.uuid}. Expected: #{self.expected_total_number_of_parts}, Got: #{total_number_of_parts}"
          )

          false

        self.expected_total_number_of_parts < current_part_number ->
          Logger.error(
            "Incorrect part number for part #{current_part_number} of message #{self.uuid}. It should less or equal than #{total_number_of_parts}"
          )

          false

        Map.has_key?(self.parts, current_part_number) ->
          Logger.warn(
            "The part #{current_part_number}/#{total_number_of_parts} has already been received for message #{self.uuid}}"
          )

          true

        total_number_of_parts > @max_number_of_parts ->
          Logger.error(
            "Received the part #{current_part_number}/#{total_number_of_parts} of message #{self.uuid}. The total number of parts should be less or equal to #{@max_number_of_parts}"
          )

          false

        true ->
          true
      end
    end
  end

  defmodule RefreshCredentials do
    @moduledoc """
    Secure channel message refreshing sender credentials
    """
    defstruct [:contact, :credentials]

    def from_cbor_term([change_history, credentials]) do
      {:ok,
       %RefreshCredentials{
         contact: CBOR.encode(change_history),
         credentials: Enum.map(credentials, fn c -> CBOR.encode(c) end)
       }}
    end

    def to_cbor_term(%RefreshCredentials{contact: contact, credentials: credentials}) do
      {:ok, contact, ""} = CBOR.decode(contact)

      credentials =
        Enum.map(credentials, fn c ->
          {:ok, d, ""} = CBOR.decode(c)
          d
        end)

      {:ok, [contact, credentials]}
    end
  end

  @enum_schema {:variant_enum,
                [
                  {Ockam.SecureChannel.Messages.Payload, 0},
                  {Ockam.SecureChannel.Messages.RefreshCredentials, 1},
                  {:close, 2},
                  {Ockam.SecureChannel.Messages.PayloadPart, 3}
                ]}

  def decode(encoded) do
    TypedCBOR.decode_strict(@enum_schema, encoded)
  end

  def encode(msg) do
    TypedCBOR.encode(@enum_schema, msg)
  end
end
