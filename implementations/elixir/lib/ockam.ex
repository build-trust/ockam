defmodule Ockam do
  @on_load :init

  def init do
    path = Application.app_dir(:ockam, "priv/ockam") |> String.to_charlist
    :ok = :erlang.load_nif(path, 0)
  end

  def random() do
    exit(:nif_not_loaded)
  end
end
