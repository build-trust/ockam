defmodule Ockam.Node.Web.PageController do
  use Phoenix.Controller, namespace: Ockam.Node.Web

  def index(conn, _params) do
    render(conn, "index.html", item: "100")
  end
end
