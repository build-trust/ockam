defprotocol Ockam.Router.Protocol.Encoder do
  @type t :: term()
  @type opts :: map()
  @type reason :: term()

  @fallback_to_any true

  @spec encode(t, opts) :: {:ok, iodata} | {:error, reason}
  def encode(value, opts)
end

defimpl Ockam.Router.Protocol.Encoder, for: Any do
  alias Ockam.Router.Protocol.Encoding.Default

  defmacro __deriving__(module, _struct, opts) do
    quote do
      require Protocol
      Protocol.derive(unquote(Default.Encoder), unquote(module), unquote(opts))

      defimpl Ockam.Router.Protocol.Encoder, for: unquote(module) do
        def encode(value, opts) do
          unquote(Default.Encoder).encode(value, opts)
        end
      end
    end
  end

  def encode(value, opts) do
    Default.Encoder.encode(value, opts)
  end
end
