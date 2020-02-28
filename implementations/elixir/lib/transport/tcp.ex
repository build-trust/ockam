defmodule Ockam.Transport.TCP do
  use Supervisor

  alias Ockam.Transport.Address

  defmodule Config do
    defstruct [:listen_address, :trace]

    def from_keyword(opts) when is_list(opts) do
      listen_address = Keyword.get(opts, :listen_address)
      listen_port = Keyword.get(opts, :listen_port)
      trace = Keyword.get(opts, :trace, false)

      with {:ok, addr} <- Address.new(:inet, listen_address, listen_port) do
        {:ok, %__MODULE__{listen_address: addr, trace: trace}}
      end
    end

    def debug_options(%__MODULE__{trace: true}), do: :sys.debug_options([:trace])
    def debug_options(%__MODULE__{}), do: :sys.debug_options([])
  end

  def start_link([meta, opts]) when is_list(meta) and is_list(opts) do
    name = Keyword.get(meta, :name, __MODULE__)

    with {:ok, config} <- Config.from_keyword(opts) do
      Supervisor.start_link(__MODULE__, [name, config], name: name)
    end
  end

  @impl true
  def init([name, config]) do
    sup_name = Module.concat(name, Connections)

    children = [
      {__MODULE__.Listener, [sup_name, config]},
      {__MODULE__.ConnectionSupervisor, [sup_name, config]}
    ]

    Supervisor.init(children, strategy: :rest_for_one)
  end
end
