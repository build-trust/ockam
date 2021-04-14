defmodule Ockam.Protocol do
  @moduledoc false
  @enforce_keys [:name]
  defstruct [:name, :request, :response]

  ## TODO: schema type. Should be in bare.erl
  @type bare_schema() :: atom() | tuple()

  @type extended_schema() :: bare_schema() | [{atom(), bare_schema()}]

  @type t() :: %__MODULE__{
          name: String.t(),
          request: extended_schema() | nil,
          response: extended_schema() | nil
        }

  @type schema_map() :: %{String.t() => extended_schema()}

  @type mapping() :: %{
          in: schema_map(),
          out: schema_map()
        }

  @callback protocol() :: __MODULE__.t()

  def client(protocol) do
    mapping([{:client, protocol}])
  end

  def server(protocol) do
    mapping([{:server, protocol}])
  end

  def mapping(protocol_specs) do
    protocol_specs = expand_specs(protocol_specs)
    check_conflicts(protocol_specs)

    Enum.reduce(protocol_specs, %{in: %{}, out: %{}}, fn
      {:client, protocol}, %{in: in_map, out: out_map} ->
        name = protocol.name

        %{
          in: update_schema_map(in_map, name, protocol.response),
          out: update_schema_map(out_map, name, protocol.request)
        }

      {:server, protocol}, %{in: in_map, out: out_map} ->
        name = protocol.name

        %{
          in: update_schema_map(in_map, name, protocol.request),
          out: update_schema_map(out_map, name, protocol.response)
        }
    end)
  end

  defp expand_specs(protocol_specs) do
    Enum.map(
      protocol_specs,
      fn
        {type, module} when is_atom(module) -> {type, module.protocol()}
        {type, %Ockam.Protocol{} = protocol} -> {type, protocol}
        {type, map} when is_map(map) -> {type, struct(Ockam.Protocol, map)}
      end
    )
  end

  defp check_conflicts(protocol_specs) do
    duplicate_names =
      protocol_specs
      |> Enum.map(fn {_, protocol} -> protocol.name end)
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
end
