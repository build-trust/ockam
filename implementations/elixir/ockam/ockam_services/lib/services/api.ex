## TODO: this module can be moved to ockam application
defmodule Ockam.Services.API do
  @moduledoc """
  Ockam request-response API service behaviour

  Defines a behaviour and `use` macro to create API workers

  Usage:
  defmodule MyAPI do
    use Ockam.Services.API

    @impl true
    def handle_request(request, state) do
      {:reply, status, body, state}
    end
  end
  """

  alias Ockam.API.Request
  alias Ockam.API.Response

  alias Ockam.Message
  alias Ockam.Router
  alias Ockam.Telemetry

  require Logger

  @doc """
  Function to handle API requests.

  Should return:
  {:reply, status, body, state} - send a response with status and body
  {:noreply, state} - don't send any responses
  {:error, reason} - send an error response with status and body derived from reason
  """
  @callback handle_request(request :: %Request{}, state :: map()) ::
              {:reply, status :: any(), body :: any(), state :: map()}
              | {:noreply, state :: map()}
              | {:error, reason :: any(), state :: map()}
              | {:error, reason :: any()}

  @doc """
  Function to simplify path value for metrics reporting.
  Due to dynamic nature of path, reporting it in metrics may cause
  high cardinality issues.
  Because path format is defined by the API worker implementation, only id can
  simplofy the path.

  Defaults to "all"
  """
  @callback path_group(String.t()) :: String.t()

  def path_group(_path), do: "all"

  @doc """
  Send Ockam.API.Response to the from_route of the reply using address as a return_route
  """
  def reply(%Request{} = request, status, body, address, module \\ __MODULE__) do
    response = Response.reply_to(request, status_code(status), body)
    reply_message = Response.to_message(response, [address])
    Router.route(reply_message)
    emit_request_stop(module, request, {:reply, response}, address)
    {:reply, response}
  end

  @doc """
  Send Ockam.API.Response to the from_route of the reply
  with an error status and body created from `reason` using address as a return_route
  """
  def reply_error(request, reason, address, module \\ __MODULE__) do
    status = status_code(reason)
    body = CBOR.encode(error_message(reason))
    reply(request, status, body, address, module)
  end

  def handle_message(module, message, state) do
    case Request.from_message(message) do
      {:ok, request} ->
        start_time = emit_request_start(module, request, state.address)
        request = %{request | start_time: start_time}

        {_reply, state} = handle_request(module, request, state)

        {:ok, state}

      {:error, {:decode_error, reason, data}} ->
        Logger.debug("Decode error: cannot decode request #{data}: #{inspect(reason)}")
        reply = reply_error(message, {:bad_request, :decode_error}, state.address, module)
        emit_decode_error(message, reply, state)
        {:ok, state}
    end
  end

  def handle_request(module, request, state) do
    case module.handle_request(request, state) do
      {:reply, status, body, state} ->
        reply = reply(request, status, body, state.address, module)
        {reply, state}

      {:noreply, state} ->
        ## No response, but still emit reply=false stop event
        emit_request_stop(module, request, :noreply, state.address)
        {:noreply, state}

      {:error, reason, state} ->
        ## TODO: handle errors differently to return error response
        reply = reply_error(request, reason, state.address, module)
        {reply, state}

      {:error, reason} ->
        ## TODO: handle errors differently to return error response
        reply = reply_error(request, reason, state.address, module)
        {reply, state}
    end
  end

  @doc """
  Get an integer status code from a status (atom or an error reason)
  """
  def status_code(code) when is_integer(code) do
    code
  end

  def status_code(:ok) do
    200
  end

  def status_code(:method_not_allowed) do
    405
  end

  def status_code(:bad_request) do
    400
  end

  def status_code({:bad_request, _reason}) do
    400
  end

  def status_code({:decode_error, _reason, _data}) do
    400
  end

  def status_code(:unauthorized) do
    401
  end

  def status_code({:unauthorized, _reason}) do
    401
  end

  def status_code(:not_found) do
    404
  end

  def status_code(:resource_exists) do
    409
  end

  def status_code({:resource_exists, _reason}) do
    409
  end

  def status_code(_unknown_error) do
    500
  end

  ## TODO: better standard error messages
  def error_message({:bad_request, data}) do
    error_message(data)
  end

  def error_message({:decode_error, reason, _data}) do
    error_message(reason)
  end

  def error_message({:unauthorized, data}) do
    error_message(data)
  end

  def error_message({:resource_exists, data}) do
    error_message(data)
  end

  def error_message(message) when is_binary(message) do
    message
  end

  def error_message(message) do
    ## TODO: better standard error messages
    inspect(message)
  end

  defmacro __using__(_options) do
    quote do
      use Ockam.Worker
      alias Ockam.API.Request
      alias Ockam.API.Response

      alias Ockam.Services.API

      @behaviour Ockam.Services.API

      @impl true
      def handle_message(message, state) do
        API.handle_message(__MODULE__, message, state)
      end

      @impl Ockam.Services.API
      defdelegate path_group(path), to: API

      defoverridable path_group: 1
    end
  end

  ## Metrics helpers

  @handle_request_event [:api, :handle_request]

  defp emit_request_start(module, request, address) do
    request_metadata = request_metadata(module, request)
    state_metadata = %{address: address}
    metadata = Map.merge(request_metadata, state_metadata)
    Telemetry.emit_start_event(@handle_request_event, metadata: metadata)
  end

  defp emit_request_stop(module, request, reply, address) do
    request_metadata = request_metadata(module, request)
    reply_metadata = reply_metadata(reply)
    state_metadata = %{address: address}
    start_time = request.start_time || System.monotonic_time()
    metadata = request_metadata |> Map.merge(reply_metadata) |> Map.merge(state_metadata)
    Telemetry.emit_stop_event(@handle_request_event, start_time, metadata: metadata)
  end

  defp emit_decode_error(message, reply, address) do
    message_metadata = message_metadata(message)
    reply_metadata = reply_metadata(reply)
    state_metadata = %{address: address}
    metadata = message_metadata |> Map.merge(reply_metadata) |> Map.merge(state_metadata)
    Telemetry.emit_event(@handle_request_event ++ [:decode_error], metadata: metadata)
  end

  defp request_metadata(module, %Request{} = request) do
    path_group = module.path_group(request.path)

    %{
      path_group: path_group,
      method: request.method,
      from_route: request.from_route
    }
  end

  defp reply_metadata(:noreply) do
    %{reply: false}
  end

  defp reply_metadata({:reply, %Response{status: status}}) do
    %{status: status, reply: true}
  end

  defp message_metadata(message) do
    return_route = Message.return_route(message)
    %{from_route: return_route}
  end
end
