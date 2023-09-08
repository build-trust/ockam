Nonterminals link_or_image
             link rest
             inside_brackets inside_brackets_part anything.

Terminals any_quote open_bracket open_title close_bracket open_paren close_paren verbatim escaped exclamation_mark ws.

Rootsymbol link_or_image.

link_or_image -> exclamation_mark link : make_image_tuple('$2').
link_or_image -> link : '$1'.

link -> open_bracket close_bracket                      : {link, "", "[]"}.
link -> open_bracket close_bracket rest                 : {link, "", "[]"}.
link -> open_bracket inside_brackets close_bracket      : title_tuple('$2').
link -> open_bracket inside_brackets close_bracket rest : title_tuple('$2').

inside_brackets -> inside_brackets_part                       : '$1'.
inside_brackets -> inside_brackets_part inside_brackets       : concat_tuple('$1', '$2').

inside_brackets_part -> exclamation_mark                           : extract_token('$1').
inside_brackets_part -> verbatim                                   : extract_token('$1').
inside_brackets_part -> ws                                         : extract_token('$1').
inside_brackets_part -> open_title                                 : extract_token('$1').
inside_brackets_part -> open_paren                                 : {"(", "("}.
inside_brackets_part -> close_paren                                : {")", ")"}.
inside_brackets_part -> any_quote                                  : extract_token('$1').
inside_brackets_part -> escaped                                    : escaped_token('$1').
inside_brackets_part -> open_bracket close_bracket                 : {"[]", "[]"}.
inside_brackets_part -> open_bracket inside_brackets close_bracket : concat_3t("[", '$2', "]").

rest     -> anything.
rest     -> anything rest.

anything -> exclamation_mark.
anything -> ws.
anything -> verbatim.
anything -> open_paren.
anything -> close_paren.
anything -> open_bracket.
anything -> close_bracket.
anything -> any_quote.
anything -> escaped.
anything -> open_title.

Erlang code.

concat_tuple({LT, LP}, {RT, RP}) -> {string:concat(LT, RT), string:concat(LP, RP)}.

concat_3t(L, {MT, MP}, R) -> {string:join([L, MT, R], ""), string:join([ L, MP, R ], "")}.

escaped_token({_Token, _Line, Value}) -> {string:concat("\\", Value), string:concat("\\", Value)}.

extract_token({_Token, _Line, Value}) -> {Value, Value}.

make_image_tuple({_Link, L, R}) -> {image, L, string:concat("!", R)}.

title_tuple({Title, Parsed}) -> {link, Title, string:join(["[", Parsed, "]"], "")}.

%% SPDX-License-Identifier: Apache-2.0
