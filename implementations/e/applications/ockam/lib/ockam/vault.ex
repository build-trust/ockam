defmodule Ockam.Vault do
  @moduledoc false

  @type t :: any()
  @type digest :: <<_::32>>

  @doc """
  Computes the SHA-256 digest of the input.
  """
  @callback sha256(vault :: t, input :: binary()) ::
              {:ok, digest :: digest()} | {:error, reason :: any()}

  @doc """
  Dispatches to an im .
  """
  @spec sha256(vault :: t, input :: binary()) ::
          {:ok, digest :: digest()} | {:error, reason :: any()}
  def sha256(vault, input), do: vault.sha256(input)
end
