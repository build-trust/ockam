defmodule Ockam.Router.Protocol.Message do
  alias Ockam.Router.Protocol.Decoder
  alias Ockam.Router.Protocol.DecodeError
  alias Ockam.Router.Protocol.MessageType

  @type t :: %{__struct__: module | map()}
  @type version :: non_neg_integer()
  @type opts :: map()

  @callback type_id() :: non_neg_integer()
  @callback version() :: non_neg_integer()
  @callback decode(version, term, opts) :: {:ok, t} | {:error, DecodeError.t() | Exception.t()}
  @callback handle_version_change(version, term, opts) ::
              {:ok, t} | {:error, DecodeError.t() | Exception.t()}

  defmodule UnknownVersionError do
    defexception [:type, :version, :description]

    def message(%__MODULE__{description: description}), do: description
  end

  defmacro __using__(opts \\ []) do
    caller = __CALLER__.module
    schema = Keyword.get(opts, :schema, [])
    version = Keyword.get(opts, :version, 1)
    typeid = Keyword.fetch!(opts, :type_id)

    keys =
      Enum.map(schema, fn
        {key, {_type, default}} ->
          {key, default}

        {key, [_type]} ->
          {key, []}

        {key, _type} ->
          {key, nil}
      end)

    derivation =
      case Keyword.get(opts, :derive, true) do
        opt when opt in [true, :all] ->
          quote do
            @derive {Ockam.Router.Protocol.Encoder, schema: unquote(schema)}
            @derive {Ockam.Router.Protocol.Decoder, schema: unquote(schema)}
          end

        :encoder ->
          quote do
            @derive {Ockam.Router.Protocol.Encoder, schema: unquote(schema)}
          end

        :decoder ->
          quote do
            @derive {Ockam.Router.Protocol.Decoder, schema: unquote(schema)}
          end

        _ ->
          quote(do: nil)
      end

    quote location: :keep do
      @behaviour unquote(__MODULE__)

      @version unquote(version)

      unquote(derivation)
      defstruct unquote(keys)

      alias Ockam.Router.Protocol.DecodeError

      @impl unquote(__MODULE__)
      def type_id, do: unquote(typeid)

      @impl unquote(__MODULE__)
      def version, do: unquote(version)

      @impl unquote(__MODULE__)
      def decode(version, input, opts)

      def decode(@version, input, opts) when is_binary(input) do
        unquote(Decoder).decode(%__MODULE__{}, input, opts)
      end

      def decode(@version, input, _opts) do
        {:error, DecodeError.new({:invalid_message_body, __MODULE__, input})}
      end

      def decode(version, input, opts) when is_binary(input) do
        handle_version_change(version, input, opts)
      end

      @impl unquote(__MODULE__)
      def handle_version_change(version, _input, _opts) do
        raise unquote(UnknownVersionError),
          type: __MODULE__,
          version: version,
          description: """
          A message of known type was received, but with a different version than
          the currently supported version (#{@version}).

          To handle version changes for message types which are backwards or fowards
          compatible, you must implement the `handle_version_change/3` callback. By
          default, this error is raised.
          """
      end

      defoverridable decode: 3, handle_version_change: 3

      defimpl unquote(MessageType) do
        def type_id(_), do: unquote(caller).type_id()
        def version(_), do: unquote(caller).version()
      end
    end
  end

  @doc false
  @spec lookup(non_neg_integer()) ::
          {:ok, module()} | {:error, {:unknown_type_id, non_neg_integer()}}
  def lookup(id) when is_integer(id) do
    search_paths = [:code.lib_dir(:ockam, :ebin)]
    impls = Protocol.extract_impls(MessageType, search_paths)

    lookup(impls, id)
  end

  defp lookup([], id), do: {:error, {:unknown_type_id, id}}

  defp lookup([mod | rest], id) do
    case mod.type_id() do
      ^id ->
        {:ok, mod}

      _ ->
        lookup(rest, id)
    end
  end
end
