defmodule Ockam.Protocol.Mapping do
  @moduledoc """
  Protocol mapping helper module to encode and decode messages
  with different protocols

  Usage:

  use Ockam.Protocol.Mapping

  @impl true
  def protocol_mapping() do
    Ockam.Protocol.Mapping.mapping([
      {:client, Protocol1},
      {:server, Protocol2}
    ])
  end

  def handle_message(%{payload: payload}, state) do
    case decode_payload(payload) do
      {:ok, Protocol1, message} ->
        #response from Protocol1
      {:ok, Protocol2, message} ->
        #request from Protocol1
    end
  end
  """
  alias Ockam.Bare.Extended, as: BareExtended
  alias Ockam.Protocol

  @type extended_schema() :: BareExtended.extended_schema()
  @type schema_map() :: %{String.t() => extended_schema()}

  @type mapping() :: %{
          in: schema_map(),
          out: schema_map(),
          modules: %{String.t() => module()}
        }

  def client(protocol) do
    mapping([{:client, protocol}])
  end

  def server(protocol) do
    mapping([{:server, protocol}])
  end

  def mapping(protocol_specs) do
    protocol_specs = expand_specs(protocol_specs)
    check_conflicts(protocol_specs)

    protocol_modules =
      Enum.reduce(
        protocol_specs,
        %{},
        fn {_, mod, protocol}, mod_map ->
          Map.put(mod_map, protocol.name, mod)
        end
      )

    Enum.reduce(protocol_specs, %{in: %{}, out: %{}, modules: protocol_modules}, fn
      {:client, _mod, protocol}, %{in: in_map, out: out_map, modules: modules} ->
        name = protocol.name

        %{
          in: update_schema_map(in_map, name, protocol.response),
          out: update_schema_map(out_map, name, protocol.request),
          modules: modules
        }

      {:server, _mod, protocol}, %{in: in_map, out: out_map, modules: modules} ->
        name = protocol.name

        %{
          in: update_schema_map(in_map, name, protocol.request),
          out: update_schema_map(out_map, name, protocol.response),
          modules: modules
        }
    end)
  end

  @type protocol_id() :: module() | String.t()

  @spec decode_payload(binary(), mapping()) :: {:ok, protocol_id(), any()} | {:error, any()}
  def decode_payload(data, mapping) do
    in_map = mapping.in

    case Protocol.base_decode(data) do
      {:ok, %{protocol: name, data: protocol_data}} ->
        with {:ok, schema} <- Map.fetch(in_map, name),
             {:ok, decoded} <- BareExtended.decode(protocol_data, schema) do
          protocol_id = protocol_id(mapping, name)
          {:ok, protocol_id, decoded}
        else
          :error ->
            {:error, {:unmatched_protocol, name, mapping}}

          other ->
            other
        end

      other ->
        other
    end
  end

  @spec encode_payload(protocol_id(), any(), mapping()) :: binary()
  def encode_payload(module, data, mapping) when is_atom(module) do
    name = Map.fetch!(module.protocol(), :name)
    encode_payload(name, data, mapping)
  end

  def encode_payload(name, data, mapping) when is_binary(name) do
    out_map = mapping.out

    case Map.fetch(out_map, name) do
      {:ok, schema} ->
        encoded = BareExtended.encode(data, schema)
        Protocol.base_encode(name, encoded)

      :error ->
        :erlang.error({:error, {:unmatched_protocol, name, mapping}})
    end
  end

  defp protocol_id(mapping, name) do
    case Map.get(mapping.modules, name) do
      nil -> name
      module -> module
    end
  end

  defp expand_specs(protocol_specs) do
    Enum.map(
      protocol_specs,
      fn
        {type, module} when is_atom(module) -> {type, module, module.protocol()}
        {type, %Protocol{} = protocol} -> {type, nil, protocol}
        {type, map} when is_map(map) -> {type, nil, struct(Protocol, map)}
      end
    )
  end

  defp check_conflicts(protocol_specs) do
    duplicate_names =
      protocol_specs
      |> Enum.map(fn {_, _, protocol} -> protocol.name end)
      |> Enum.frequencies()
      |> Enum.filter(fn {_k, v} -> v > 1 end)
      |> Enum.map(fn {k, _v} -> k end)

    case duplicate_names do
      [] ->
        :ok

      _list ->
        raise(
          "Protocol name conflict in #{inspect(protocol_specs)}. Duplicate names: #{
            inspect(duplicate_names)
          }"
        )
    end
  end

  @spec update_schema_map(schema_map(), String.t(), extended_schema() | nil) :: schema_map()
  defp update_schema_map(map, _name, nil) do
    map
  end

  defp update_schema_map(map, name, schema) do
    Map.put(map, name, schema)
  end

  ## Behaviour

  @callback protocol_mapping() :: mapping()

  defmacro __using__(_options) do
    alias Ockam.Protocol.Mapping

    quote do
      @behaviour Ockam.Protocol.Mapping

      def decode_payload(payload) do
        mapping = protocol_mapping()
        Mapping.decode_payload(payload, mapping)
      end

      def encode_payload(type, option, data) do
        encode_payload(type, {option, data})
      end

      def encode_payload(type, data) do
        mapping = protocol_mapping()

        Mapping.encode_payload(type, data, mapping)
      end
    end
  end
end
