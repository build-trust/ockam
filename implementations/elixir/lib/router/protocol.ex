defmodule Ockam.Router.Protocol do
  alias __MODULE__.Encoding
  alias __MODULE__.{EncodeError, DecodeError}

  @type message :: %{__struct__: module | map()}
  @type opts :: map()

  @spec encode_message(message, opts) :: {:ok, binary} | {:error, EncodeError.t() | Exception.t()}
  defdelegate encode_message(message, opts \\ %{}), to: Encoding, as: :encode

  @spec decode_message(iodata, opts) ::
          {:ok, message, binary} | {:error, DecodeError.t() | Exception.t()}
  def decode_message(input, opts \\ %{keys: :atoms!})

  def decode_message(input, opts) when is_list(input) do
    decode_message(IO.iodata_to_binary(input), opts)
  end

  def decode_message(input, opts) when is_binary(input) do
    Encoding.decode(input, normalize_opts(opts))
  end

  defp normalize_opts(opts) when is_map(opts), do: opts
  defp normalize_opts(opts) when is_list(opts), do: Enum.into(opts, %{})
end
