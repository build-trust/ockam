defmodule Ockam.Vault.Secret do
  alias Ockam.Vault.SecretAttributes

  defstruct [:secret, :attrs]

  @opaque t :: %__MODULE__{}

  @doc false
  def new(secret, %SecretAttributes{} = attrs) do
    %__MODULE__{secret: secret, attrs: attrs}
  end
end
