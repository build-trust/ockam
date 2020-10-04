defmodule Ockam.Error do
  @moduledoc false

  defmacro __using__(_options) do
    quote do
      @type t :: %__MODULE__{reason: any(), module: atom}

      defexception [:reason, :module]

      def message(%__MODULE__{reason: reason, module: module}) do
        module.format_error(reason)
      end
    end
  end
end
