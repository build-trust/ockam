defmodule Ockam.Kafka.Interceptor do
  @moduledoc """
  Implementation of Ockam.Transport.Portal.Interceptor callbacks
  to intercept kafka protocol messages.

  Supports pluggable handlers for requests and responses.

  Used with `Ockam.Kafka.Interceptor.MetadataHandler` to intercept metadata messages

  Options:
  :request_handlers - handler functions for requests (see `Ockam.Kafka.Interceptor.MetadataHandler`)
  :response_handlers - handler functions for responses
  :handler_options - additional options to be used by handlers
  """

  @behaviour Ockam.Transport.Portal.Interceptor

  alias Ockam.Kafka.Interceptor.Protocol.Parser

  alias Ockam.Kafka.Interceptor.Protocol.RequestHeader

  alias Ockam.Kafka.Interceptor.Protocol.Metadata.Response, as: MetadataResponse

  require Logger

  @impl true
  def setup(options, state) do
    handler_options = Keyword.get(options, :handler_options, [])
    request_handlers = Keyword.get(options, :request_handlers, [])
    response_handlers = Keyword.get(options, :response_handlers, [])
    correlations = %{}

    {:ok,
     Map.put(state, :correlations, correlations)
     |> Map.put(:request_handlers, request_handlers)
     |> Map.put(:response_handlers, response_handlers)
     |> Map.put(:handler_options, handler_options)
     |> Map.put(:read_buffer, <<>>)}
  end

  ## Kafka requests
  @impl true
  def handle_outer_payload(tunnel_payload, state) do
    process_kafka_packet(:request, tunnel_payload, state)
  end

  ## Kafka responses
  @impl true
  def handle_inner_payload(tunnel_payload, state) do
    process_kafka_packet(:response, tunnel_payload, state)
  end

  @impl true
  def handle_outer_signal(_signal, state) do
    {:ok, state}
  end

  @impl true
  def handle_inner_signal(_signal, state) do
    {:ok, state}
  end

  ## TODO: limit read buffer to 32Mb
  @spec process_kafka_packet(
          type :: :request | :response,
          packet :: binary(),
          state :: map(),
          replies :: [binary()]
        ) ::
          {:ok, replies :: [binary()], state :: map()}
          | {:error, reason :: any(), replies :: [binary()], state :: map()}
  defp process_kafka_packet(type, packet, state, replies \\ [])

  defp process_kafka_packet(type, packet, %{read_buffer: <<>>} = state, replies) do
    case packet do
      <<size::signed-big-integer-size(32), data::binary-size(size), rest::binary>> ->
        ## There is enough data in the packet to read the message
        case process_kafka_message(type, data, state) do
          {:ok, new_data, state} ->
            size = byte_size(new_data)

            process_kafka_packet(
              type,
              rest,
              state,
              replies ++ [<<size::signed-big-integer-size(32), new_data::binary>>]
            )

          {:error, reason, state} ->
            {:stop, reason, replies, state}
        end

      <<>> ->
        ## We finished reading the packet
        {:ok, replies, state}

      <<_size::signed-big-integer-size(32), _rest::binary>> ->
        ## There is not enough data in the packed
        ## return replies and wait for the next packet
        {:ok, replies, %{state | read_buffer: packet}}

      other ->
        ## We couldn't read the size bytes
        {:stop, {:cannot_parse_message_size, other}, replies, state}
    end
  end

  defp process_kafka_packet(type, packet, %{read_buffer: read_buffer} = state, replies) do
    process_kafka_packet(type, read_buffer <> packet, %{state | read_buffer: <<>>}, replies)
  end

  @spec process_kafka_message(type :: :request | :response, message :: binary(), state :: map()) ::
          {:ok, message :: binary(), state :: map()} | {:error, reason :: any(), state :: map()}
  defp process_kafka_message(type, message, state) do
    result =
      case type do
        :request -> process_kafka_request(message, state)
        :response -> process_kafka_response(message, state)
      end

    case result do
      {:ok, new_payload, new_state} ->
        {:ok, new_payload, new_state}

      {:error, reason} ->
        handle_error(reason, message, type, state)

      {:error, reason, state} ->
        handle_error(reason, message, type, state)
    end
  end

  defp handle_error({:unsupported_api, _api}, message, _type, state) do
    {:ok, message, state}
  end

  defp handle_error(
         {:correlation_id_not_found, _correlation_id, _response} = reason,
         _message,
         _type,
         state
       ) do
    {:error, reason, state}
  end

  defp handle_error({:unsupported_api_version, _api} = reason, _message, _type, state) do
    {:error, reason, state}
  end

  defp handle_error(reason, message, type, state) do
    Logger.warning(
      "Kafka interceptor processing error for type: #{inspect(type)} : #{inspect(reason)}"
    )

    ## Tunnel messages should still be forwarded even if processing failed
    {:ok, message, state}
  end

  @spec process_kafka_response(binary(), state :: map()) ::
          {:ok, binary(), state :: map()} | {:error, reason :: any(), state :: map()}
  defp process_kafka_response(response, state) do
    case Parser.parse_response_correlation_id(response) do
      {:ok, correlation_id, _rest} ->
        case get_request_header(correlation_id, state) do
          {:ok, request_header} ->
            state = cleanup_request_header(correlation_id, state)
            ## Return error with state to cleanup request header for this correlation id
            with {:error, reason} <-
                   process_kafka_response_for_request(request_header, response, state) do
              {:error, reason, state}
            end

          {:error, :not_found} ->
            {:error, {:correlation_id_not_found, correlation_id, response}, state}

          {:error, :not_tracked} ->
            Logger.info("Correlation id not tracked")
            {:ok, response, state}
        end

      {:error, _reason} ->
        {:error, {:response_header_error, :cannot_parse_correlation_id, response}, state}
    end
  end

  @spec process_kafka_response_for_request(RequestHeader.t(), binary(), state :: map()) ::
          {:ok, binary(), state :: map()} | {:error, reason :: any()}
  defp process_kafka_response_for_request(request_header, response, state) do
    case Parser.parse_kafka_response_for_request(request_header, response) do
      {:ok, response_header, response_content_size, response_content} ->
        case handle_kafka_response(response_header, response_content, state) do
          {:ok, state} ->
            {:ok, response, state}

          {:ok, updated_response_content, state} ->
            ## Simplify reconstruction of response header by using the original header binary
            ## since we don't change the header we can just reuse it with new response content
            old_header_binary_size = byte_size(response) - response_content_size
            <<old_header::binary-size(old_header_binary_size), _response_data::binary>> = response

            reconstruct_response(old_header, updated_response_content, state)

          {:error, reason} ->
            {:error, reason}
        end

      {:error, reason} ->
        {:error, reason}
    end
  end

  @spec process_kafka_request(binary(), state :: map()) ::
          {:ok, binary(), state :: map()}
          | {:error, reason :: any(), state :: map()}
  defp process_kafka_request(request, state) do
    case Parser.parse_kafka_request(request) do
      {:ok, request_header, request_content_size, request_content} ->
        state = save_request_header(request_header, state)

        case handle_kafka_request(request_header, request_content, state) do
          {:ok, state} ->
            {:ok, request, state}

          {:ok, updated_request_content, state} ->
            old_header_binary_size = byte_size(request) - request_content_size
            <<old_header::binary-size(old_header_binary_size), _request_data::binary>> = request

            case reconstruct_request(old_header, updated_request_content) do
              {:ok, new_request} -> {:ok, new_request, state}
              {:error, reason} -> {:error, reason, state}
            end

          {:error, reason} ->
            {:error, reason, state}
        end

      {:error, {:unsupported_api, api, %RequestHeader{} = header}} ->
        state = mark_untracked_header(header, state)
        {:error, {:unsupported_api, api}, state}

      {:error, reason} ->
        {:error, reason, state}
    end
  end

  defp handle_kafka_response(
         response_header,
         response_content,
         %{response_handlers: handlers} = state
       ) do
    Enum.reduce(handlers, {:ok, state}, fn
      handler, {:ok, state} -> handler.(response_header, response_content, state)
      handler, {:ok, prev_response, state} -> handler.(response_header, prev_response, state)
      _handler, {:error, reason} -> {:error, reason}
    end)
  end

  defp handle_kafka_request(
         request_header,
         request_content,
         %{request_handlers: handlers} = state
       ) do
    Enum.reduce(handlers, {:ok, state}, fn
      handler, {:ok, state} -> handler.(request_header, request_content, state)
      handler, {:ok, prev_request, state} -> handler.(request_header, prev_request, state)
      _handler, {:error, reason} -> {:error, reason}
    end)
  end

  ## Currently we don't modify requests, hence we don't support reconstructing form struct
  @spec reconstruct_request(binary(), request :: any()) ::
          {:ok, binary()} | {:error, reason :: any()}
  defp reconstruct_request(_header_binary, request) do
    {:error, {:unsupported_request, request}}
  end

  @spec reconstruct_response(binary(), response :: any(), state :: map()) ::
          {:ok, binary(), state :: map()} | {:error, reason :: any()}
  defp reconstruct_response(header_binary, %MetadataResponse{} = response, state) do
    case MetadataResponse.Formatter.format(response) do
      {:ok, response_binary} ->
        {:ok, header_binary <> response_binary, state}

      {:error, reason} ->
        {:error, reason}
    end
  end

  defp reconstruct_response(_header_binary, response, _state) do
    {:error, {:unsupported_response, response}}
  end

  ## Correlation ID tracking

  defp save_request_header(
         %RequestHeader{correlation_id: correlation_id} = header,
         %{correlations: correlations} = state
       ) do
    Map.put(state, :correlations, Map.put(correlations, correlation_id, header))
  end

  defp mark_untracked_header(
         %RequestHeader{
           api_key: api_key,
           api_version: api_version,
           correlation_id: correlation_id
         },
         %{correlations: correlations} = state
       ) do
    Logger.info(
      "Not tracking requests for api_key: #{inspect(api_key)} api_version: #{inspect(api_version)}"
    )

    Map.put(state, :correlations, Map.put(correlations, correlation_id, :untracked))
  end

  defp get_request_header(correlation_id, %{correlations: correlations}) do
    case Map.fetch(correlations, correlation_id) do
      :error -> {:error, :not_found}
      {:ok, :untracked} -> {:error, :not_tracked}
      {:ok, %RequestHeader{} = header} -> {:ok, header}
    end
  end

  defp cleanup_request_header(correlation_id, %{correlations: correlations} = state) do
    Map.put(state, :correlations, Map.delete(correlations, correlation_id))
  end
end
