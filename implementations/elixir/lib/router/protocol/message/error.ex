defmodule Ockam.Router.Protocol.Message.Error do
  use Ockam.Router.Protocol.Message,
    type_id: 6,
    schema: [code: :integer, description: :string]

  def new(code, reason) when is_integer(code) and is_binary(reason) do
    %__MODULE__{code: code, description: reason}
  end

  def new(code, reason) when is_integer(code) do
    %__MODULE__{code: code, description: to_string(reason)}
  rescue
    Protocol.UndefinedError ->
      %__MODULE__{code: code, description: inspect(reason)}
  end
end
