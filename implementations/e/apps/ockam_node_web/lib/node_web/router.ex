defmodule Ockam.Node.Web.Router do
  use Plug.Router
  require EEx

  # Returns a specification to start this module under a supervisor. When this
  # module is added to a supervisor, the supervisor calls child_spec to figure
  # out the specification that should be used.
  #
  # See the "Child specification" section in the `Supervisor` module for more
  # detailed information.
  @doc false
  def child_spec(_options) do
    Plug.Cowboy.child_spec(
      scheme: :http,
      plug: __MODULE__,
      options: [
        dispatch: dispatch(),
        port: 4000
      ]
    )
  end

  defp dispatch do
    [
      {:_,
       [
         {"/ws/[...]", Ockam.Node.Web.Socket, []},
         {:_, Plug.Cowboy.Handler, {__MODULE__, []}}
       ]}
    ]
  end

  plug(Plug.Static,
    at: "/",
    from: :ockam_node_web
  )

  plug(:match)
  plug(:dispatch)

  EEx.function_from_file(:defp, :index_html, "lib/node_web/templates/index.html.eex", [])
  EEx.function_from_file(:defp, :error_html, "lib/node_web/templates/error.html.eex", [])

  get "/" do
    send_resp(conn, 200, index_html())
  end

  match _ do
    send_resp(conn, 404, error_html())
  end
end
