defmodule Ockam.Identity.SecureChannel.ServiceMessage do
  @moduledoc """
  Service message for identity secure channel.

  Currently supports :disconnect
  """
  use TypedStruct

  typedstruct do
    plugin(Ockam.TypedCBOR.Plugin)
    field(:command, :disconnect, minicbor: [key: 1, schema: {:enum, [disconnect: 0]}])
  end
end
