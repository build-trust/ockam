defprotocol Ockam.Router.Protocol.MessageType do
  @type t :: any()

  @spec type_id(t) :: non_neg_integer()
  def type_id(value)

  @spec version(t) :: non_neg_integer()
  def version(value)
end
