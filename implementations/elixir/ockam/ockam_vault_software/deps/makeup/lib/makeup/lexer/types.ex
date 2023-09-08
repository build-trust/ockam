defmodule Makeup.Lexer.Types do
  @type token :: {atom(), Map.t(), iodata()}
  @type tokens :: [token()]
  @type context :: Map.t()
  @type parsec_success :: {:ok, tokens, String.t(), context(), integer(), integer()}
  @type parsec_failure :: {:error, String.t(), Sring.t(), context(), {integer(), integer()}, integer()}
  @type parsec_result :: parsec_success | parsec_failure
  @type parsec :: (String.t -> parsec_result)
end

