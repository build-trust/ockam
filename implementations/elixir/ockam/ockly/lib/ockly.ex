defmodule Ockly do
  @moduledoc """
  Documentation for `Ockly`.
  """

  def nif_config do
    case Application.fetch_env(:ockly, :aws_vault) do
      {:ok, true} -> :aws_kms
      _ -> nil
    end
  end
end
