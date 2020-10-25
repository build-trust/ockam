defmodule Ockam.Error do
  @moduledoc false

  defmacro __using__(_options) do
    quote do
      @type t :: %__MODULE__{reason: any()}

      defexception [:reason, :metadata]

      def message(error), do: format_error(error)

      def format_error(%__MODULE__{metadata: %{module: module, stacktrace: stacktrace}} = error) do
        module.format_error(error) <> "\nStacktrace:\n" <> Exception.format_stacktrace(stacktrace)
      end

      defmacro new(reason) do
        error_module = __MODULE__
        caller_module = __CALLER__.module

        quote do
          {:current_stacktrace, [_ | stacktrace]} = Process.info(self(), :current_stacktrace)

          metadata = %{stacktrace: stacktrace, module: unquote(caller_module)}
          %unquote(error_module){reason: unquote(reason), metadata: metadata}
        end
      end
    end
  end
end
