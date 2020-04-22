defmodule Ockam.Transport.Address do
  defstruct [:family, :addr, :port]

  defmodule InvalidAddressError do
    defexception [:message]

    def new({:unsupported_family, family}) do
      %__MODULE__{message: "unsupported address family: #{inspect(family)}"}
    end

    def new(reason) when not is_binary(reason) do
      %__MODULE__{message: "invalid address: #{inspect(reason)}"}
    end

    def new(reason) when is_binary(reason) do
      %__MODULE__{message: reason}
    end

    def message(%__MODULE__{message: message}), do: message
  end

  def new!(family, addr, port \\ nil)

  def new!(family, addr, port) do
    case new(family, addr, port) do
      {:ok, addr} ->
        addr

      {:error, reason} ->
        raise InvalidAddressError.new(reason)
    end
  end

  def new(family, addr, port \\ nil)

  def new(:inet, addr, port) when addr in [:any, :loopback] do
    case parse_port(port) do
      {:ok, port} ->
        {:ok, %__MODULE__{family: :inet, addr: addr, port: port}}

      {:error, _reason} = err ->
        err
    end
  end

  def new(:inet, addr, port) do
    with {:ok, addr} <- parse_address(addr),
         {:ok, port} <- parse_port(port) do
      {:ok, %__MODULE__{family: :inet, addr: addr, port: port}}
    end
  end

  def new(family, _addr, _port) do
    {:error, {:unsupported_family, family}}
  end

  def ip(%__MODULE__{addr: :any}), do: {0, 0, 0, 0}
  def ip(%__MODULE__{addr: :loopback}), do: {127, 0, 0, 1}
  def ip(%__MODULE__{addr: {_, _, _, _} = addr}), do: addr
  def ip(%__MODULE__{addr: {_, _, _, _, _, _, _, _} = addr}), do: addr

  def port(%__MODULE__{port: port}), do: port

  @doc """
  Converts this struct to the Erlang address representation
  expected by the `:socket` API.
  """
  def to_erl(%__MODULE__{} = addr) do
    Map.from_struct(addr)
  end

  def parse_address(:any), do: :any
  def parse_address(:loopback), do: :loopback

  def parse_address(addr) when is_binary(addr) do
    parse_address(String.to_charlist(addr))
  end

  def parse_address(addr) when is_list(addr) do
    case :inet.parse_address(addr) do
      {:ok, _addr} = result ->
        result

      {:error, _reason} = err ->
        err
    end
  end

  def parse_address(addr), do: {:error, {:invalid_address, addr}}

  def parse_port(p) when is_integer(p) and p > 0 and p <= 65535 do
    {:ok, p}
  end

  def parse_port(p) when is_binary(p), do: parse_port(String.to_integer(p))
  def parse_port(p), do: {:error, {:invalid_port, p}}
end
