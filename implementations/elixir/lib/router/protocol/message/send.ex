defmodule Ockam.Router.Protocol.Message.Send do
  use Ockam.Router.Protocol.Message,
    type_id: 7,
    schema: [data: :raw]
end
