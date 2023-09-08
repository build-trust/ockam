defmodule EarmarkParser.Error do

  @moduledoc false

  defexception [:message]

  @doc false
  def exception(msg), do: %__MODULE__{message: msg}

end

# SPDX-License-Identifier: Apache-2.0
