defmodule Ockam.Pipeline do
  @moduledoc false

  defmacro __using__(_options) do
    quote do
      @behaviour Ockam.Worker

      def init(options \\ []), do: pipeline_init(options)
      def handle(message, options \\ []), do: pipeline_handle(message, options)

      import Ockam.Pipeline, only: [step: 1, step: 2]
      Module.register_attribute(__MODULE__, :steps, accumulate: true)
      @before_compile Ockam.Pipeline
    end
  end

  @doc false
  defmacro __before_compile__(macro_environment) do
    macro_environment.module
    |> Module.get_attribute(:steps)
    |> Enum.reverse()
    |> compile
  end

  defmacro step(name, options \\ []) do
    name = Macro.expand(name, %{__CALLER__ | function: {:init, 1}})
    type = detect_step_type(name)

    quote do
      @steps {unquote(type), unquote(name), unquote(options), true}
    end
  end

  defp detect_step_type(step_name) do
    case Atom.to_charlist(step_name) do
      ~c"Elixir." ++ _rest -> :module
      _something_else -> :function
    end
  end

  defp compile(steps) do
    [compile_pipeline_init(steps), compile_pipeline_handle(steps)]
  end

  defp compile_pipeline_init(steps) do
    body_ast = Enum.map(steps, &quote_step_init/1)

    quote do
      defp pipeline_init(_options) do
        unquote(body_ast)
        :ok
      end
    end
  end

  defp compile_pipeline_handle(steps) do
    message = quote do: message
    body_ast = Enum.reduce(steps, message, &quote_step_handle/2)

    quote do
      defp pipeline_handle(message, state), do: unquote(body_ast)
    end
  end

  # for a module step, invoke init function of module
  defp quote_step_init({:module, name, options, _guards}) do
    quote do: unquote(name).init(unquote(options))
  end

  # for a function step, do nothing to initialize.
  defp quote_step_init({:function, _name, _options, _guards}) do
    []
  end

  # for a module step, invoke handle function of module
  defp quote_step_handle({:module, name, options, _guards}, message) do
    quote do: unquote(name).handle(unquote(message), unquote(options))
  end

  # for a function step, invoke the function
  defp quote_step_handle({:function, name, options, _guards}, message) do
    quote do: unquote(name)(unquote(message), unquote(options))
  end
end
