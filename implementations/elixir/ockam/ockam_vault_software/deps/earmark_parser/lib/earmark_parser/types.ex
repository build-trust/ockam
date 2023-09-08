defmodule EarmarkParser.Types do

  @moduledoc false

  defmacro __using__(_options \\ []) do
    quote do
      @type token  :: {atom, String.t}
      @type tokens :: list(token)
      @type numbered_line :: %{required(:line) => String.t, required(:lnb) => number, optional(:inside_code) => String.t}
      @type message_type :: :warning | :error
      @type message :: {message_type, number, String.t}
      @type maybe(t) :: t | nil
      @type inline_code_continuation :: {nil | String.t, number}
    end
  end

end

# SPDX-License-Identifier: Apache-2.0
