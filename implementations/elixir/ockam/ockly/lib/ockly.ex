defmodule Ockly do
  @moduledoc """
  Documentation for `Ockly`.
  """

  def nif_config do
    case Application.fetch_env(:ockly, :aws_vault) do
      {:ok, true} -> :aws_kms
      _ -> :aws_kms
    end
  end
end
