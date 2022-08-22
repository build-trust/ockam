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

  alias Ockam.Router

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
        with {:ok, request} <- Request.from_message(message) do
          case handle_request(request, state) do
            {:reply, status, body, state} ->
              :ok = API.reply(request, status, body, state.address)
              {:ok, state}

            {:noreply, state} ->
              {:ok, state}

            {:error, reason, state} ->
              ## TODO: handle errors differently to return error response
              :ok = API.reply_error(request, reason, state.address)
              {:ok, state}

            {:error, reason} ->
              ## TODO: handle errors differently to return error response
              :ok = API.reply_error(request, reason, state.address)
              {:ok, state}
          end

          ## TODO: handle failure to parse a request
        end
      end

      def setup_handler(_options, state), do: {:ok, state}

      defoverridable setup_handler: 2
    end
  end
end
