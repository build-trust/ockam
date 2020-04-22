defmodule Ockam.Router.Protocol.Message.Connect do
  use Ockam.Router.Protocol.Message,
    type_id: 4,
    schema: [options: [Ockam.Router.Protocol.Message.Connect.Option]]

  defmodule Option do
    @schema [name: :string, value: :string]
    @derive {Ockam.Router.Protocol.Encoder, schema: @schema}
    @derive {Ockam.Router.Protocol.Decoder, schema: @schema}
    defstruct [:name, :value]
  end
end
