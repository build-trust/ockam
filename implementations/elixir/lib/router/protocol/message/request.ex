defmodule Ockam.Router.Protocol.Message.Request do
  use Ockam.Router.Protocol.Message,
    type_id: 8,
    schema: [data: :raw]
end
