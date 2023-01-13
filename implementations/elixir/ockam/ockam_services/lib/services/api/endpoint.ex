defmodule Ockam.Services.API.Endpoint do
  defmodule DispatchTable do
    def compile_dispatch_table(rules) do
      catch_all_rule = {:_, :not_found, %{}}

      case Enum.all?(rules, fn
             {_auth_type, method, <<"/", _rest::binary>> = _path, handler}
             when method in [:get, :post, :put, :delete] and
                    is_function(handler, 3) ->
               true

             _bad_rule ->
               false
           end) do
        true ->
          table =
            Enum.reduce(rules, %{}, fn {auth_type, method, <<"/", _rest::binary>> = path, handler},
                                       acc ->
              rule = {path, handler, %{auth_type: auth_type}}
              Map.update(acc, method, [rule], &[rule | &1])
            end)

          {:ok,
           table
           |> Enum.map(fn {method, rules} ->
             {method, :cowboy_router.compile([{:_, Enum.reverse([catch_all_rule | rules])}])}
           end)
           |> Enum.into(%{})}

        false ->
          {:error, :bad_dispatch_rules}
      end
    end

    # Paths *must* start with a "/" (due to how cowboy' routing works)
    def match(dispatch_table, method, <<"/", _rest::binary>> = path) do
      case Map.get(dispatch_table, method) do
        nil ->
          :error

        table ->
          req = %{host: "controller", path: path}

          case :cowboy_router.execute(req, %{dispatch: table}) do
            {:ok, %{}, %{handler: :not_found}} ->
              :error

            {:ok, %{bindings: bindings}, %{handler: handler, handler_opts: opts}} ->
              {:ok, handler, bindings, Map.get(opts, :auth_type)}
          end
      end
    end

    def match(_dispatch_table, _method, _path) do
      :error
    end
  end

  @callback routes() :: [
              {auth_type :: any(), method :: atom(), path :: String.t(), handler :: atom()}
            ]

  @callback authorize(auth_type :: any(), req :: %Ockam.API.Request{}, bindings :: map()) ::
              true | false | {true, values :: map()}

  defmacro __using__(_options) do
    quote do
      @doc false
      use Ockam.Services.API

      @behaviour Ockam.Services.API.Endpoint

      alias Ockam.API.Request
      alias Ockam.Services.API.Endpoint.DispatchTable

      @impl true
      def handle_request(%Request{method: method, path: path} = req, state) do
        case DispatchTable.match(
               state.dispatch_table,
               method,
               path
             ) do
          {:ok, handler, bindings, auth_type} ->
            case authorize(auth_type, req, bindings) do
              false ->
                {:error, 401}

              true ->
                dispatch(handler, req, bindings, %{}, state)

              {true, extra_data} ->
                dispatch(handler, req, bindings, extra_data, state)

              {:error, reason} ->
                {:error, reason}
            end

          :error ->
            {:error, :not_found}
        end
      end

      @impl true
      def setup(_options, state) do
        routes = routes()

        case DispatchTable.compile_dispatch_table(routes) do
          {:ok, dispatch_table} ->
            {:ok, Map.put(state, :dispatch_table, dispatch_table)}

          error ->
            error
        end
      end

      defp dispatch(handler, req, bindings, extra_data, state) do
        case handler.(req, bindings, extra_data) do
          {:ok, body} ->
            {:reply, :ok, body, state}

          {:error, reason} ->
            {:error, reason}
        end
      end

      @impl true
      defp authorize(_auth_type, _req, _bindings), do: true

      defoverridable authorize: 3
    end
  end
end
