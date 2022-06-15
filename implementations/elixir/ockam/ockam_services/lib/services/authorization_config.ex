defmodule Ockam.Services.AuthorizationConfig do
  @moduledoc """
  Predefined configs for services authorization
  """
  def secure_channel() do
    [:to_my_address, :is_secure]
  end

  def local() do
    [:to_my_address, :is_local]
  end
end
