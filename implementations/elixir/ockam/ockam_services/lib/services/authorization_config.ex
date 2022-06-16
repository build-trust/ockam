defmodule Ockam.Services.AuthorizationConfig do
  @moduledoc """
  Predefined configs for services authorization
  """
  def secure_channel() do
    [:to_my_address, :from_secure_channel]
  end

  def identity_secure_channel() do
    [:to_my_address, :from_identiy_secure_channel]
  end

  def local() do
    [:to_my_address, :is_local]
  end
end
