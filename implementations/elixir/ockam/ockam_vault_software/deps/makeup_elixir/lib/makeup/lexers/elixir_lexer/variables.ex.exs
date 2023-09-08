defmodule Makeup.Lexers.ElixirLexer.Variables do
  @moduledoc false

  # parsec:Makeup.Lexers.ElixirLexer.Variables
  # This module is generated at "dev time" so that the lexer
  # doesn't have to depend on the (excellent) `unicode_set` library,
  # which takes several minutes to compile.
  import NimbleParsec

  variable_start_unicode_syntax =
    "[[:L:][:Nl:][:Other_ID_Start:]-[:Pattern_Syntax:]-[:Pattern_White_Space:]-[:Lu:]-[:Lt:][_]]"

  variable_continue_unicode_syntax =
    "[[:ID_Start:][:Mn:][:Mc:][:Nd:][:Pc:][:Other_ID_Continue:]-[:Pattern_Syntax:]-[:Pattern_White_Space:]]"

  # TODO: Why do we need to flatten these lists? A bug in `unicode_set`?
  variable_start_chars = Unicode.Set.to_utf8_char(variable_start_unicode_syntax) |> List.flatten()

  variable_continue_chars =
    Unicode.Set.to_utf8_char(variable_continue_unicode_syntax) |> List.flatten()

  defcombinator(:variable_start_chars, label(utf8_char(variable_start_chars), "variable start"))

  defcombinator(
    :variable_continue_chars,
    label(utf8_char(variable_continue_chars), "variable continue")
  )

  # parsec:Makeup.Lexers.ElixirLexer.Variables
end
