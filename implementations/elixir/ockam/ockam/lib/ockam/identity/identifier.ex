defmodule Ockam.Identity.Identifier do
  @moduledoc false

  alias __MODULE__

  defstruct [:id]

  @type t :: %Identifier{id: binary()}

  def from_cbor_term(%CBOR.Tag{tag: :bytes, value: id}), do: {:ok, %Identifier{id: id}}
  def to_cbor_term(%Identifier{id: id}), do: {:ok, %CBOR.Tag{tag: :bytes, value: id}}

  def to_str(%Identifier{id: id}) do
    "I" <> Base.encode16(id, case: :lower)
  end

  def from_str(<<"I", hex::binary-size(64)>>) do
    {:ok, id} = Base.decode16(hex, case: :lower)
    %Identifier{id: id}
  end
end
