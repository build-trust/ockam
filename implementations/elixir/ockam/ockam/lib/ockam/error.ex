defmodule Ockam.Error do
  @moduledoc false

  defmacro __using__(_options) do
    quote do
      @type t :: %__MODULE__{reason: any(), module: atom}

      defexception [:reason, :module]

      def message(%__MODULE__{module: module} = error) do
        module.format_error(error)
      end
    end
  end
end
