Definitions.

WHITESPACE=[\s\t\r\n]+
SCIENTIFIC_NOTATION = -?[0-9]+\.[0-9]+e-?[0-9]+
INT = -?[0-9]+
REST = \.\.\.
RANGE = \.\.
ATOM = \'[^']+\'
WHEN = \swhen\s

Rules.

{WHITESPACE} : skip_token.

{REST} : {token, {'...', TokenLine}}.
{WHEN}  : {token, {'when', TokenLine}}.
fun\( : {token, {'fun(',  TokenLine}}.
\* : {token, {'*',  TokenLine}}.
\[ : {token, {'[',  TokenLine}}.
\] : {token, {']',  TokenLine}}.
\( : {token, {'(',  TokenLine}}.
\) : {token, {')',  TokenLine}}.
\{ : {token, {'{',  TokenLine}}.
\} : {token, {'}',  TokenLine}}.
\# : {token, {'#',  TokenLine}}.
\| : {token, {'|',  TokenLine}}.
_ : {token, {'_',  TokenLine}}.
\:\: : {token, {'::',  TokenLine}}.
\: : {token, {':',  TokenLine}}.
\:\= : {token, {':=',  TokenLine}}.
\=\> : {token, {'=>',  TokenLine}}.
\-\> : {token, {'->',  TokenLine}}.
\| : {token, {'|',  TokenLine}}.
\< : {token, {'<', TokenLine}}.
\> : {token, {'>', TokenLine}}.
\' : {token, {'\'',  TokenLine}}.
, : {token, {',',  TokenLine}}.
\= : {token, {'=',  TokenLine}}.
{RANGE} : {token, {'..', TokenLine}}.
{SCIENTIFIC_NOTATION} : {token, {int,  TokenLine, TokenChars}}.
{INT} : {token, {int,  TokenLine, list_to_integer(TokenChars)}}.
{ATOM} : {token, {atom_full, TokenLine, TokenChars}}.
. : {token, {atom_part, TokenLine, TokenChars}}.

Erlang code.
