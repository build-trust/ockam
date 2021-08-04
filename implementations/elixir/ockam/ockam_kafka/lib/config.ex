defmodule Ockam.Kafka.Config do
  @moduledoc """
  Generate :brod configuration from :ockam_kafke environment variables
  """

  def replication_factor(args) do
    Keyword.get(
      args,
      :replication_factor,
      Application.get_env(:ockam_kafka, :replication_factor)
    )
  end

  def stream_prefix(args) do
    stream_prefix =
      Keyword.get(args, :stream_prefix, Application.get_env(:ockam_kafka, :stream_prefix, ""))

    case stream_prefix do
      "" -> ""
      string -> "#{string}_"
    end
  end

  def client_config(args) do
    sasl_options = sasl_options(args)
    ssl = Keyword.get(args, :ssl, Application.get_env(:ockam_kafka, :ssl))

    [ssl: ssl] ++ sasl_options
  end

  def sasl_options(args) do
    sasl =
      args |> Keyword.get(:sasl, Application.get_env(:ockam_kafka, :sasl)) |> String.to_atom()

    user = Keyword.get(args, :user, Application.get_env(:ockam_kafka, :user))
    password = Keyword.get(args, :password, Application.get_env(:ockam_kafka, :password))

    case user do
      nil ->
        []

      _defined ->
        [sasl: {sasl, user, password}]
    end
  end

  def endpoints(args) do
    args
    |> Keyword.get(:endpoints, Application.get_env(:ockam_kafka, :endpoints))
    |> parse_endpoints()
  end

  def parse_endpoints(endpoints) when is_list(endpoints) do
    Enum.map(endpoints, fn string when is_binary(string) ->
      with [host, port_str] <- String.split(string, ":"),
           port_int <- String.to_integer(port_str) do
        {host, port_int}
      else
        err ->
          raise("Unable to parse kafka endpoints: #{inspect(endpoints)}: #{inspect(err)}")
      end
    end)
  end

  def parse_endpoints(endpoints) do
    parse_endpoints(String.split(endpoints, ","))
  end
end
