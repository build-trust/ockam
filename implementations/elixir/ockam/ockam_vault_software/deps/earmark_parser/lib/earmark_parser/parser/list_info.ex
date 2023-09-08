defmodule EarmarkParser.Parser.ListInfo do
  import EarmarkParser.Helpers.LookaheadHelpers, only: [opens_inline_code: 1, still_inline_code: 2]

  @moduledoc false

  @not_pending {nil, 0}

  defstruct(
    indent: 0,
    pending: @not_pending,
    spaced: false,
    width: 0)

  # INLINE CANDIDATE
  def new(%EarmarkParser.Line.ListItem{initial_indent: ii, list_indent: width}=item) do
    pending = opens_inline_code(item)
    %__MODULE__{indent: ii, pending: pending, width: width}
  end

  # INLINE CANDIDATE
  def update_pending(list_info, line)
  def update_pending(%{pending: @not_pending}=info, line) do
    pending = opens_inline_code(line)
    %{info | pending: pending}
  end
  def update_pending(%{pending: pending}=info, line) do
    pending1 = still_inline_code(line, pending)
    %{info | pending: pending1}
  end
end
