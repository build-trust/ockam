Definitions.

OTHER       = [^`\\]+
ESCAPE      = \\
BACKTIX     = `+

Rules.

{OTHER}   : {token, {other, TokenLine, TokenChars}}.
{ESCAPE}  : {token, {escape, TokenLine, TokenChars}}.
{BACKTIX} : {token, {backtix, TokenLine, TokenChars}}.

Erlang code.
