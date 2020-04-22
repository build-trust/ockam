defprotocol Ockam.Router.Protocol.Decoder do
  @type t :: term()
  @type key :: atom()
  @type type :: :raw | :string | :boolean | :integer | module()
  @type descriptor :: type | {type, default_value :: term()}
  @type field :: {key, descriptor}
  @type schema :: [field]
  @type opt :: {:schema, schema}
  @type opts :: [opt]
  @type reason :: term()

  @fallback_to_any true

  @spec decode(t, iodata, opts) :: {:ok, t, iodata} | {:error, reason}
  def decode(value, encoded, opts)
end

defimpl Ockam.Router.Protocol.Decoder, for: Any do
  alias Ockam.Router.Protocol.Encoding.Default

  defmacro __deriving__(module, _struct, opts) do
    quote do
      require Protocol
      Protocol.derive(unquote(Default.Decoder), unquote(module), unquote(opts))

      defimpl Ockam.Router.Protocol.Decoder, for: unquote(module) do
        def decode(value, encoded, opts) do
          unquote(Default.Decoder).decode(value, encoded, opts)
        end
      end
    end
  end

  def decode(value, encoded, opts) do
    Default.Decoder.decode(value, encoded, opts)
  end
end
