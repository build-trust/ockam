defmodule Dialyxir.Warning do
  @moduledoc """
  Behaviour for defining warning semantics.

  Contains callbacks for various warnings
  """

  @doc """
  By expressing the warning that is to be matched on, error handling
  and dispatching can be avoided in format functions.
  """
  @callback warning() :: atom

  @doc """
  The default documentation when seeing an error without the user
  otherwise overriding the format.
  """
  @callback format_long([String.t()] | {String.t(), String.t(), String.t()} | String.t()) ::
              String.t()

  @doc """
  A short message, often missing things like success types and expected types for space.
  """
  @callback format_short([String.t()] | {String.t(), String.t(), String.t()} | String.t()) ::
              String.t()

  @doc """
  Explanation for a warning of this type. Should include a simple example of how to trigger it.
  """
  @callback explain() :: String.t()

  @spec default_explain() :: String.t()
  def default_explain() do
    """
    This warning type does not have an explanation yet. If you have
    code that causes it, please file an issue or pull request in
    https://github.com/jeremyjh/dialyxir/issues
    """
  end
end
