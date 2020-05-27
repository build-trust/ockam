defmodule Ockam.Transport.TCP do
  @moduledoc """
  Defines an Ockam TCP Transport
  """

  def start_link do
    :ranch.start_listener(
      make_ref(),
      :ranch_tcp,
      [{:port, 5555}],
      __MODULE__.Server.Connection,
      []
    )
  end
end
