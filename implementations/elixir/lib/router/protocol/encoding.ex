defmodule Ockam.Router.Protocol.Encoding do
  @type message :: term
  @type opts :: map()
  @type reason :: term

  alias Ockam.Router.Protocol.{EncodeError, DecodeError}
  alias __MODULE__.Default

  @callback encode!(message, opts) :: binary | no_return
  @callback encode(message, opts) :: {:ok, binary} | {:error, EncodeError.t() | Exception.t()}
  @callback decode!(iodata, opts) :: {message, binary} | no_return
  @callback decode(iodata, opts) ::
              {:ok, message, binary} | {:error, DecodeError.t() | Exception.t()}

  defdelegate encode!(message, opts \\ %{}), to: Default
  defdelegate encode(message, opts \\ %{}), to: Default
  defdelegate decode!(encoded, opts \\ %{}), to: Default
  defdelegate decode(encoded, opts \\ %{}), to: Default
end
