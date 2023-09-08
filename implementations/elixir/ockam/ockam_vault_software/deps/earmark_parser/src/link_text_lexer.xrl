Definitions.

ESCAPED          = \\.
ESCAPE           = \\
EXCLAMATION_MARK = [!]
OPEN_PAREN       = \(
CLOSE_PAREN      = \)
OPEN_BRACKET     = \[
CLOSE_BRACKET    = \]
OPEN_TITLE       = \s+['"]
ANY_QUOTE        = ['"]
WS               = \s+
ANY              = [^]\\"'()[\s]+

Rules.

{ESCAPED}          : {token, {escaped, TokenLine, dismiss_backslash(TokenChars)}}.
{EXCLAMATION_MARK} : {token, {exclamation_mark, TokenLine, TokenChars}}.
{OPEN_PAREN}       : {token, {open_paren, TokenLine, TokenChars}}.
{CLOSE_PAREN}      : {token, {close_paren, TokenLine, TokenChars}}.
{OPEN_BRACKET}     : {token, {open_bracket, TokenLine, TokenChars}}.
{CLOSE_BRACKET}    : {token, {close_bracket, TokenLine, TokenChars}}.
{OPEN_TITLE}       : {token, {open_title, TokenLine, TokenChars}}.
{ANY_QUOTE}        : {token, {any_quote, TokenLine, TokenChars}}.
{ESCAPE}           : {token, {verbatim, TokenLine, TokenChars}}.
{WS}               : {token, {ws, TokenLine, TokenChars}}.
{ANY}              : {token, {verbatim, TokenLine, TokenChars}}.

Erlang code.

dismiss_backslash([$\\|Chars]) -> Chars.
