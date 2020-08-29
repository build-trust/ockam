defmodule Ockam.Node.Web.Endpoint do
  use Phoenix.Endpoint, otp_app: :ockam_node_web

  plug(Plug.Static,
    at: "/",
    from: :ockam_node_web,
    gzip: false,
    only: ~w(css fonts images js favicon.ico robots.txt)
  )

  plug(Ockam.Node.Web.Router)
end
