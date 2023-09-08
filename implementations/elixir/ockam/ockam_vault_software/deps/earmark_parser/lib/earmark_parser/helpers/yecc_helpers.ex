defmodule EarmarkParser.Helpers.YeccHelpers do

  @moduledoc false
  import EarmarkParser.Helpers.LeexHelpers, only: [lex: 2]

  def parse!( text, lexer: lexer, parser: parser ) do
    case parse(text, lexer: lexer, parser: parser) do
        {:ok, ast}  -> ast
        {:error, _} -> nil
    end
  end

  def parse( text, lexer: lexer, parser: parser ) do
    with tokens <- lex(text, with: lexer) do
      parser.parse(tokens)
    end
  end
end

# SPDX-License-Identifier: Apache-2.0
