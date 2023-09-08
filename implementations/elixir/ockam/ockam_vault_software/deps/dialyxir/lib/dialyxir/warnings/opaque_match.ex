defmodule Dialyxir.Warnings.OpaqueMatch do
  @moduledoc """
  Attempted to pattern match against the internal structure of an
  opaque term.

  ## Example

      defmodule OpaqueStruct do
        defstruct [:opaque]

        @opaque t :: %__MODULE__{}

        @spec opaque() :: t
        def opaque() do
          %__MODULE__{}
        end
      end

      defmodule Example do
        @spec error() :: :error
        def error() do
          %{opaque: _} = OpaqueStruct.opaque()
          :error
        end
      end
  """

  @behaviour Dialyxir.Warning

  @impl Dialyxir.Warning
  @spec warning() :: :opaque_match
  def warning(), do: :opaque_match

  @impl Dialyxir.Warning
  @spec format_short([String.t()]) :: String.t()
  def format_short([_pattern, type | _]) do
    pretty_type = Erlex.pretty_print_type(type)

    "Attempted to pattern match against the internal structure of an opaque term of type #{
      pretty_type
    }."
  end

  @impl Dialyxir.Warning
  @spec format_long([String.t()]) :: String.t()
  def format_long([pattern, opaque_type, opaque_term]) do
    pretty_opaque_term = Erlex.pretty_print(opaque_term)

    term =
      if opaque_type == opaque_term do
        "the term"
      else
        pretty_opaque_term
      end

    pretty_pattern = Erlex.pretty_print_pattern(pattern)

    """
    Attempted to pattern match against the internal structure of an opaque term.

    Type:
    #{pretty_opaque_term}

    Pattern:
    #{pretty_pattern}

    This breaks the opaqueness of #{term}.
    """
  end

  @impl Dialyxir.Warning
  @spec explain() :: String.t()
  def explain() do
    @moduledoc
  end
end
