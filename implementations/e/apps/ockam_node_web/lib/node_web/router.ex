defmodule Ockam.Node.Web.Router do
  use Phoenix.Router

  scope "/", Ockam.Node.Web do
    get("/", PageController, :index)
  end
end
