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
  Send Ockam.API.Response to the from_route of the reply using address as a return_route
  """
  def reply(%Request{} = request, status, body, address) do
    response = Response.reply_to(request, status_code(status), body)
    reply_message = Response.to_message(response, [address])
    Router.route(reply_message)
    {:reply, response}
  end

  @doc """
  Send Ockam.API.Response to the from_route of the reply
  with an error status and body created from `reason` using address as a return_route
  """
  def reply_error(request, reason, address) do
    status = status_code(reason)
    body = CBOR.encode(error_message(reason))
    reply(request, status, body, address)
  end

  def handle_message(module, message, state) do
    case Request.from_message(message) do
      {:ok, request} ->
        start_time = emit_request_start(request, state)

        {reply, state} = handle_request(module, request, state)

        emit_request_stop(start_time, request, reply, state)

        {:ok, state}

      {:error, {:decode_error, reason, data}} ->
        Logger.debug("Decode error: cannot decode request #{data}: #{inspect(reason)}")
        reply = reply_error(message, {:bad_request, :decode_error}, state.address)
        emit_decode_error(message, reply, state)
        {:ok, state}
    end
  end

  def handle_request(module, request, state) do
    case module.handle_request(request, state) do
      {:reply, status, body, state} ->
        reply = reply(request, status, body, state.address)
        {reply, state}

      {:noreply, state} ->
        {:noreply, state}

      {:error, reason, state} ->
        ## TODO: handle errors differently to return error response
        reply = reply_error(request, reason, state.address)
        {reply, state}

      {:error, reason} ->
        ## TODO: handle errors differently to return error response
        reply = reply_error(request, reason, state.address)
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

      def setup_handler(_options, state), do: {:ok, state}

      defoverridable setup_handler: 2
    end
  end

  ## Metrics helpers

  @handle_request_event [:api, :handle_request]

  defp emit_request_start(request, state) do
    request_metadata = request_metadata(request)
    state_metadata = state_metadata(state)
    metadata = Map.merge(request_metadata, state_metadata)
    Telemetry.emit_start_event(@handle_request_event, metadata: metadata)
  end

  defp emit_request_stop(start_time, request, reply, state) do
    request_metadata = request_metadata(request)
    reply_metadata = reply_metadata(reply)
    state_metadata = state_metadata(state)
    metadata = request_metadata |> Map.merge(reply_metadata) |> Map.merge(state_metadata)
    Telemetry.emit_stop_event(@handle_request_event, start_time, metadata: metadata)
  end

  defp emit_decode_error(message, reply, state) do
    message_metadata = message_metadata(message)
    reply_metadata = reply_metadata(reply)
    state_metadata = state_metadata(state)
    metadata = message_metadata |> Map.merge(reply_metadata) |> Map.merge(state_metadata)
    Telemetry.emit_event(@handle_request_event ++ [:decode_error], metadata: metadata)
  end

  defp state_metadata(state) do
    %{address: state.address}
  end

  defp request_metadata(%Request{} = request) do
    %{path: request.path, method: request.method, from_route: request.from_route}
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
