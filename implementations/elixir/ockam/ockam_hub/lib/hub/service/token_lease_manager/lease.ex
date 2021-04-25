defmodule Ockam.TokenLeaseManager.Lease do
  
  defstruct [
    id: "",
    issued: nil,
    renewable: false,
    tags: [],
    ttl: 0,
    value: ""
  ]

end
