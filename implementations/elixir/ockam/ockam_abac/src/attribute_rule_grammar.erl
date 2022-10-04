-module(attribute_rule_grammar).
-export([parse/1,file/1]).
-define(p_anything,true).
-define(p_charclass,true).
-define(p_choose,true).
-define(p_label,true).
-define(p_not,true).
-define(p_one_or_more,true).
-define(p_optional,true).
-define(p_scan,true).
-define(p_seq,true).
-define(p_string,true).
-define(p_zero_or_more,true).



-spec file(file:name()) -> any().
file(Filename) -> case file:read_file(Filename) of {ok,Bin} -> parse(Bin); Err -> Err end.

-spec parse(binary() | list()) -> any().
parse(List) when is_list(List) -> parse(unicode:characters_to_binary(List));
parse(Input) when is_binary(Input) ->
  _ = setup_memo(),
  Result = case 'expression'(Input,{{line,1},{column,1}}) of
             {AST, <<>>, _Index} -> AST;
             Any -> Any
           end,
  release_memo(), Result.

-spec 'expression'(input(), index()) -> parse_result().
'expression'(Input, Index) ->
  p(Input, Index, 'expression', fun(I,D) -> (p_choose([fun 'rule'/2, fun 'comment'/2]))(I,D) end, fun(Node, Idx) ->transform('expression', Node, Idx) end).

-spec 'comment'(input(), index()) -> parse_result().
'comment'(Input, Index) ->
  p(Input, Index, 'comment', fun(I,D) -> (p_seq([p_string(<<";;">>), p_zero_or_more(p_anything())]))(I,D) end, fun(Node, Idx) ->transform('comment', Node, Idx) end).

-spec 'rule'(input(), index()) -> parse_result().
'rule'(Input, Index) ->
  p(Input, Index, 'rule', fun(I,D) -> (p_seq([p_zero_or_more(fun 'space'/2), p_choose([fun 'filter_rule'/2, fun 'member_rule'/2, fun 'exists_rule'/2, fun 'logical_rule'/2, fun 'boolean'/2]), p_zero_or_more(fun 'space'/2)]))(I,D) end, fun(Node, _Idx) ->[_Space1, Rule, _Space2] = Node, Rule end).

-spec 'filter_rule'(input(), index()) -> parse_result().
'filter_rule'(Input, Index) ->
  p(Input, Index, 'filter_rule', fun(I,D) -> (p_seq([fun 'open'/2, fun 'filter'/2, fun 'attribute'/2, p_choose([fun 'attribute'/2, fun 'value'/2]), fun 'close'/2]))(I,D) end, fun(Node, _Idx) ->[_Open, Filter, Attribute, Comp, _Close] = Node, {Filter, Attribute, Comp} end).

-spec 'filter'(input(), index()) -> parse_result().
'filter'(Input, Index) ->
  p(Input, Index, 'filter', fun(I,D) -> (p_seq([p_choose([p_string(<<"=">>), p_string(<<"<">>), p_string(<<">">>), p_string(<<"!=">>)]), p_zero_or_more(fun 'space'/2)]))(I,D) end, fun(Node, _Idx) ->[Filter, _Space] = Node,
case Filter of
  <<"=">> -> 'eq';
  <<"!=">> -> 'neq';
  <<">">> -> 'gt';
  <<"<">> -> 'lt'
end end).

-spec 'member_rule'(input(), index()) -> parse_result().
'member_rule'(Input, Index) ->
  p(Input, Index, 'member_rule', fun(I,D) -> (p_seq([fun 'open'/2, p_choose([p_string(<<"member?">>), p_string(<<"in">>)]), p_zero_or_more(fun 'space'/2), p_choose([fun 'attribute'/2, fun 'value'/2]), p_choose([fun 'attribute'/2, fun 'value_list'/2]), fun 'close'/2]))(I,D) end, fun(Node, _Idx) ->[_Open, _Member, _Space, Attr, List, _Close] = Node, {member, Attr, List} end).

-spec 'exists_rule'(input(), index()) -> parse_result().
'exists_rule'(Input, Index) ->
  p(Input, Index, 'exists_rule', fun(I,D) -> (p_seq([fun 'open'/2, p_string(<<"exists?">>), p_zero_or_more(fun 'space'/2), fun 'attribute'/2, fun 'close'/2]))(I,D) end, fun(Node, _Idx) ->[_Open, _Exists, _Space, Attr, _Close] = Node, {exists, Attr} end).

-spec 'logical_rule'(input(), index()) -> parse_result().
'logical_rule'(Input, Index) ->
  p(Input, Index, 'logical_rule', fun(I,D) -> (p_choose([fun 'combination_rule'/2, fun 'not_rule'/2, fun 'if_rule'/2]))(I,D) end, fun(Node, Idx) ->transform('logical_rule', Node, Idx) end).

-spec 'combination_rule'(input(), index()) -> parse_result().
'combination_rule'(Input, Index) ->
  p(Input, Index, 'combination_rule', fun(I,D) -> (p_seq([fun 'open'/2, fun 'logical_op'/2, fun 'rule'/2, p_one_or_more(fun 'rule'/2), fun 'close'/2]))(I,D) end, fun(Node, _Idx) ->[_Open, Comb, Rule, Rules, _Close] = Node, {Comb, [Rule | Rules]} end).

-spec 'logical_op'(input(), index()) -> parse_result().
'logical_op'(Input, Index) ->
  p(Input, Index, 'logical_op', fun(I,D) -> (p_choose([fun 'and'/2, fun 'or'/2]))(I,D) end, fun(Node, Idx) ->transform('logical_op', Node, Idx) end).

-spec 'and'(input(), index()) -> parse_result().
'and'(Input, Index) ->
  p(Input, Index, 'and', fun(I,D) -> (p_seq([p_string(<<"and">>), p_zero_or_more(fun 'space'/2)]))(I,D) end, fun(_Node, _Idx) ->'and' end).

-spec 'or'(input(), index()) -> parse_result().
'or'(Input, Index) ->
  p(Input, Index, 'or', fun(I,D) -> (p_seq([p_string(<<"or">>), p_zero_or_more(fun 'space'/2)]))(I,D) end, fun(_Node, _Idx) ->'or' end).

-spec 'not_rule'(input(), index()) -> parse_result().
'not_rule'(Input, Index) ->
  p(Input, Index, 'not_rule', fun(I,D) -> (p_seq([fun 'open'/2, p_string(<<"not">>), p_zero_or_more(fun 'space'/2), fun 'rule'/2, fun 'close'/2]))(I,D) end, fun(Node, _Idx) ->[_Open, _Not, _Space, Rule, _Close] = Node, {'not', Rule} end).

-spec 'if_rule'(input(), index()) -> parse_result().
'if_rule'(Input, Index) ->
  p(Input, Index, 'if_rule', fun(I,D) -> (p_seq([fun 'open'/2, p_string(<<"if">>), p_zero_or_more(fun 'space'/2), fun 'rule'/2, fun 'rule'/2, fun 'rule'/2, fun 'close'/2]))(I,D) end, fun(Node, _Idx) ->[_Open, _If, _Space, CondRule, TrueRule, FalseRule, _Close] = Node,
{'if', CondRule, TrueRule, FalseRule} end).

-spec 'attribute'(input(), index()) -> parse_result().
'attribute'(Input, Index) ->
  p(Input, Index, 'attribute', fun(I,D) -> (p_seq([fun 'type'/2, p_string(<<".">>), fun 'name'/2, p_zero_or_more(fun 'space'/2)]))(I,D) end, fun(Node, _Idx) ->[Type, _, Name, _] = Node, {binary_to_atom(Type), iolist_to_binary(Name)} end).

-spec 'type'(input(), index()) -> parse_result().
'type'(Input, Index) ->
  p(Input, Index, 'type', fun(I,D) -> (p_choose([p_string(<<"action">>), p_string(<<"subject">>), p_string(<<"resource">>)]))(I,D) end, fun(Node, Idx) ->transform('type', Node, Idx) end).

-spec 'name'(input(), index()) -> parse_result().
'name'(Input, Index) ->
  p(Input, Index, 'name', fun(I,D) -> (p_one_or_more(p_charclass(<<"[a-z0-9_]">>)))(I,D) end, fun(Node, Idx) ->transform('name', Node, Idx) end).

-spec 'value'(input(), index()) -> parse_result().
'value'(Input, Index) ->
  p(Input, Index, 'value', fun(I,D) -> (p_seq([p_choose([fun 'string'/2, fun 'boolean'/2, fun 'number'/2]), p_zero_or_more(fun 'space'/2)]))(I,D) end, fun(Node, _Idx) ->[Val, _Space] = Node, Val end).

-spec 'value_list'(input(), index()) -> parse_result().
'value_list'(Input, Index) ->
  p(Input, Index, 'value_list', fun(I,D) -> (p_seq([fun 'list_open'/2, p_zero_or_more(fun 'value'/2), fun 'list_close'/2]))(I,D) end, fun(Node, _Idx) ->[_Open, Values, _Close] = Node, Values end).

-spec 'boolean'(input(), index()) -> parse_result().
'boolean'(Input, Index) ->
  p(Input, Index, 'boolean', fun(I,D) -> (p_choose([fun 'true'/2, fun 'false'/2]))(I,D) end, fun(Node, Idx) ->transform('boolean', Node, Idx) end).

-spec 'open'(input(), index()) -> parse_result().
'open'(Input, Index) ->
  p(Input, Index, 'open', fun(I,D) -> (p_seq([p_string(<<"(">>), p_zero_or_more(fun 'space'/2)]))(I,D) end, fun(Node, Idx) ->transform('open', Node, Idx) end).

-spec 'close'(input(), index()) -> parse_result().
'close'(Input, Index) ->
  p(Input, Index, 'close', fun(I,D) -> (p_seq([p_string(<<")">>), p_zero_or_more(fun 'space'/2)]))(I,D) end, fun(Node, Idx) ->transform('close', Node, Idx) end).

-spec 'list_open'(input(), index()) -> parse_result().
'list_open'(Input, Index) ->
  p(Input, Index, 'list_open', fun(I,D) -> (p_seq([p_string(<<"[">>), p_zero_or_more(fun 'space'/2)]))(I,D) end, fun(Node, Idx) ->transform('list_open', Node, Idx) end).

-spec 'list_close'(input(), index()) -> parse_result().
'list_close'(Input, Index) ->
  p(Input, Index, 'list_close', fun(I,D) -> (p_seq([p_string(<<"]">>), p_zero_or_more(fun 'space'/2)]))(I,D) end, fun(Node, Idx) ->transform('list_close', Node, Idx) end).

-spec 'space'(input(), index()) -> parse_result().
'space'(Input, Index) ->
  p(Input, Index, 'space', fun(I,D) -> (p_choose([p_string(<<"\s">>), p_string(<<"\t">>), fun 'eol'/2]))(I,D) end, fun(Node, Idx) ->transform('space', Node, Idx) end).

-spec 'eol'(input(), index()) -> parse_result().
'eol'(Input, Index) ->
  p(Input, Index, 'eol', fun(I,D) -> (p_choose([p_string(<<"\r\n">>), p_string(<<"\n">>), p_string(<<"\r">>)]))(I,D) end, fun(Node, Idx) ->transform('eol', Node, Idx) end).

-spec 'string'(input(), index()) -> parse_result().
'string'(Input, Index) ->
  p(Input, Index, 'string', fun(I,D) -> (p_seq([p_string(<<"\"">>), p_label('chars', p_zero_or_more(p_seq([p_not(p_string(<<"\"">>)), p_choose([p_string(<<"\\\\">>), p_string(<<"\\\"">>), p_anything()])]))), p_string(<<"\"">>)]))(I,D) end, fun(Node, _Idx) ->iolist_to_binary(proplists:get_value(chars, Node)) end).

-spec 'number'(input(), index()) -> parse_result().
'number'(Input, Index) ->
  p(Input, Index, 'number', fun(I,D) -> (p_seq([fun 'int'/2, p_optional(fun 'frac'/2), p_optional(fun 'exp'/2)]))(I,D) end, fun(Node, _Idx) ->
case Node of
  [Int, [], []] -> list_to_integer(binary_to_list(iolist_to_binary(Int)));
  [Int, Frac, []] -> list_to_float(binary_to_list(iolist_to_binary([Int, Frac])));
  [Int, [], Exp] -> list_to_float(binary_to_list(iolist_to_binary([Int, ".0", Exp])));
  _ -> list_to_float(binary_to_list(iolist_to_binary(Node)))
end
 end).

-spec 'int'(input(), index()) -> parse_result().
'int'(Input, Index) ->
  p(Input, Index, 'int', fun(I,D) -> (p_choose([p_seq([p_optional(p_string(<<"-">>)), p_seq([fun 'non_zero_digit'/2, p_one_or_more(fun 'digit'/2)])]), fun 'digit'/2]))(I,D) end, fun(Node, _Idx) ->Node end).

-spec 'frac'(input(), index()) -> parse_result().
'frac'(Input, Index) ->
  p(Input, Index, 'frac', fun(I,D) -> (p_seq([p_string(<<".">>), p_one_or_more(fun 'digit'/2)]))(I,D) end, fun(Node, _Idx) ->Node end).

-spec 'exp'(input(), index()) -> parse_result().
'exp'(Input, Index) ->
  p(Input, Index, 'exp', fun(I,D) -> (p_seq([fun 'e'/2, p_one_or_more(fun 'digit'/2)]))(I,D) end, fun(Node, _Idx) ->Node end).

-spec 'e'(input(), index()) -> parse_result().
'e'(Input, Index) ->
  p(Input, Index, 'e', fun(I,D) -> (p_seq([p_charclass(<<"[eE]">>), p_optional(p_choose([p_string(<<"+">>), p_string(<<"-">>)]))]))(I,D) end, fun(Node, _Idx) ->Node end).

-spec 'non_zero_digit'(input(), index()) -> parse_result().
'non_zero_digit'(Input, Index) ->
  p(Input, Index, 'non_zero_digit', fun(I,D) -> (p_charclass(<<"[1-9]">>))(I,D) end, fun(Node, _Idx) ->Node end).

-spec 'digit'(input(), index()) -> parse_result().
'digit'(Input, Index) ->
  p(Input, Index, 'digit', fun(I,D) -> (p_charclass(<<"[0-9]">>))(I,D) end, fun(Node, _Idx) ->Node end).

-spec 'true'(input(), index()) -> parse_result().
'true'(Input, Index) ->
  p(Input, Index, 'true', fun(I,D) -> (p_seq([p_string(<<"true">>), p_zero_or_more(fun 'space'/2)]))(I,D) end, fun(_Node, _Idx) ->true end).

-spec 'false'(input(), index()) -> parse_result().
'false'(Input, Index) ->
  p(Input, Index, 'false', fun(I,D) -> (p_seq([p_string(<<"false">>), p_zero_or_more(fun 'space'/2)]))(I,D) end, fun(_Node, _Idx) ->false end).


transform(_,Node,_Index) -> Node.
-file("peg_includes.hrl", 1).
-type index() :: {{line, pos_integer()}, {column, pos_integer()}}.
-type input() :: binary().
-type parse_failure() :: {fail, term()}.
-type parse_success() :: {term(), input(), index()}.
-type parse_result() :: parse_failure() | parse_success().
-type parse_fun() :: fun((input(), index()) -> parse_result()).
-type xform_fun() :: fun((input(), index()) -> term()).

-spec p(input(), index(), atom(), parse_fun(), xform_fun()) -> parse_result().
p(Inp, StartIndex, Name, ParseFun, TransformFun) ->
  case get_memo(StartIndex, Name) of      % See if the current reduction is memoized
    {ok, Memo} -> %Memo;                     % If it is, return the stored result
      Memo;
    _ ->                                        % If not, attempt to parse
      Result = case ParseFun(Inp, StartIndex) of
        {fail,_} = Failure ->                       % If it fails, memoize the failure
          Failure;
        {Match, InpRem, NewIndex} ->               % If it passes, transform and memoize the result.
          Transformed = TransformFun(Match, StartIndex),
          {Transformed, InpRem, NewIndex}
      end,
      memoize(StartIndex, Name, Result),
      Result
  end.

-spec setup_memo() -> ets:tid().
setup_memo() ->
  put({parse_memo_table, ?MODULE}, ets:new(?MODULE, [set])).

-spec release_memo() -> true.
release_memo() ->
  ets:delete(memo_table_name()).

-spec memoize(index(), atom(), parse_result()) -> true.
memoize(Index, Name, Result) ->
  Memo = case ets:lookup(memo_table_name(), Index) of
              [] -> [];
              [{Index, Plist}] -> Plist
         end,
  ets:insert(memo_table_name(), {Index, [{Name, Result}|Memo]}).

-spec get_memo(index(), atom()) -> {ok, term()} | {error, not_found}.
get_memo(Index, Name) ->
  case ets:lookup(memo_table_name(), Index) of
    [] -> {error, not_found};
    [{Index, Plist}] ->
      case proplists:lookup(Name, Plist) of
        {Name, Result}  -> {ok, Result};
        _  -> {error, not_found}
      end
    end.

-spec memo_table_name() -> ets:tid().
memo_table_name() ->
    get({parse_memo_table, ?MODULE}).

-ifdef(p_eof).
-spec p_eof() -> parse_fun().
p_eof() ->
  fun(<<>>, Index) -> {eof, [], Index};
     (_, Index) -> {fail, {expected, eof, Index}} end.
-endif.

-ifdef(p_optional).
-spec p_optional(parse_fun()) -> parse_fun().
p_optional(P) ->
  fun(Input, Index) ->
      case P(Input, Index) of
        {fail,_} -> {[], Input, Index};
        {_, _, _} = Success -> Success
      end
  end.
-endif.

-ifdef(p_not).
-spec p_not(parse_fun()) -> parse_fun().
p_not(P) ->
  fun(Input, Index)->
      case P(Input,Index) of
        {fail,_} ->
          {[], Input, Index};
        {Result, _, _} -> {fail, {expected, {no_match, Result},Index}}
      end
  end.
-endif.

-ifdef(p_assert).
-spec p_assert(parse_fun()) -> parse_fun().
p_assert(P) ->
  fun(Input,Index) ->
      case P(Input,Index) of
        {fail,_} = Failure-> Failure;
        _ -> {[], Input, Index}
      end
  end.
-endif.

-ifdef(p_seq).
-spec p_seq([parse_fun()]) -> parse_fun().
p_seq(P) ->
  fun(Input, Index) ->
      p_all(P, Input, Index, [])
  end.

-spec p_all([parse_fun()], input(), index(), [term()]) -> parse_result().
p_all([], Inp, Index, Accum ) -> {lists:reverse( Accum ), Inp, Index};
p_all([P|Parsers], Inp, Index, Accum) ->
  case P(Inp, Index) of
    {fail, _} = Failure -> Failure;
    {Result, InpRem, NewIndex} -> p_all(Parsers, InpRem, NewIndex, [Result|Accum])
  end.
-endif.

-ifdef(p_choose).
-spec p_choose([parse_fun()]) -> parse_fun().
p_choose(Parsers) ->
  fun(Input, Index) ->
      p_attempt(Parsers, Input, Index, none)
  end.

-spec p_attempt([parse_fun()], input(), index(), none | parse_failure()) -> parse_result().
p_attempt([], _Input, _Index, Failure) -> Failure;
p_attempt([P|Parsers], Input, Index, FirstFailure)->
  case P(Input, Index) of
    {fail, _} = Failure ->
      case FirstFailure of
        none -> p_attempt(Parsers, Input, Index, Failure);
        _ -> p_attempt(Parsers, Input, Index, FirstFailure)
      end;
    Result -> Result
  end.
-endif.

-ifdef(p_zero_or_more).
-spec p_zero_or_more(parse_fun()) -> parse_fun().
p_zero_or_more(P) ->
  fun(Input, Index) ->
      p_scan(P, Input, Index, [])
  end.
-endif.

-ifdef(p_one_or_more).
-spec p_one_or_more(parse_fun()) -> parse_fun().
p_one_or_more(P) ->
  fun(Input, Index)->
      Result = p_scan(P, Input, Index, []),
      case Result of
        {[_|_], _, _} ->
          Result;
        _ ->
          {fail, {expected, Failure, _}} = P(Input,Index),
          {fail, {expected, {at_least_one, Failure}, Index}}
      end
  end.
-endif.

-ifdef(p_label).
-spec p_label(atom(), parse_fun()) -> parse_fun().
p_label(Tag, P) ->
  fun(Input, Index) ->
      case P(Input, Index) of
        {fail,_} = Failure ->
           Failure;
        {Result, InpRem, NewIndex} ->
          {{Tag, Result}, InpRem, NewIndex}
      end
  end.
-endif.

-ifdef(p_scan).
-spec p_scan(parse_fun(), input(), index(), [term()]) -> {[term()], input(), index()}.
p_scan(_, <<>>, Index, Accum) -> {lists:reverse(Accum), <<>>, Index};
p_scan(P, Inp, Index, Accum) ->
  case P(Inp, Index) of
    {fail,_} -> {lists:reverse(Accum), Inp, Index};
    {Result, InpRem, NewIndex} -> p_scan(P, InpRem, NewIndex, [Result | Accum])
  end.
-endif.

-ifdef(p_string).
-spec p_string(binary()) -> parse_fun().
p_string(S) ->
    Length = erlang:byte_size(S),
    fun(Input, Index) ->
      try
          <<S:Length/binary, Rest/binary>> = Input,
          {S, Rest, p_advance_index(S, Index)}
      catch
          error:{badmatch,_} -> {fail, {expected, {string, S}, Index}}
      end
    end.
-endif.

-ifdef(p_anything).
-spec p_anything() -> parse_fun().
p_anything() ->
  fun(<<>>, Index) -> {fail, {expected, any_character, Index}};
     (Input, Index) when is_binary(Input) ->
          <<C/utf8, Rest/binary>> = Input,
          {<<C/utf8>>, Rest, p_advance_index(<<C/utf8>>, Index)}
  end.
-endif.

-ifdef(p_charclass).
-spec p_charclass(string() | binary()) -> parse_fun().
p_charclass(Class) ->
    {ok, RE} = re:compile(Class, [unicode, dotall]),
    fun(Inp, Index) ->
            case re:run(Inp, RE, [anchored]) of
                {match, [{0, Length}|_]} ->
                    {Head, Tail} = erlang:split_binary(Inp, Length),
                    {Head, Tail, p_advance_index(Head, Index)};
                _ -> {fail, {expected, {character_class, binary_to_list(Class)}, Index}}
            end
    end.
-endif.

-ifdef(p_regexp).
-spec p_regexp(binary()) -> parse_fun().
p_regexp(Regexp) ->
    {ok, RE} = re:compile(Regexp, [unicode, dotall, anchored]),
    fun(Inp, Index) ->
        case re:run(Inp, RE) of
            {match, [{0, Length}|_]} ->
                {Head, Tail} = erlang:split_binary(Inp, Length),
                {Head, Tail, p_advance_index(Head, Index)};
            _ -> {fail, {expected, {regexp, binary_to_list(Regexp)}, Index}}
        end
    end.
-endif.

-ifdef(line).
-spec line(index() | term()) -> pos_integer() | undefined.
line({{line,L},_}) -> L;
line(_) -> undefined.
-endif.

-ifdef(column).
-spec column(index() | term()) -> pos_integer() | undefined.
column({_,{column,C}}) -> C;
column(_) -> undefined.
-endif.

-spec p_advance_index(input() | unicode:charlist() | pos_integer(), index()) -> index().
p_advance_index(MatchedInput, Index) when is_list(MatchedInput) orelse is_binary(MatchedInput)-> % strings
  lists:foldl(fun p_advance_index/2, Index, unicode:characters_to_list(MatchedInput));
p_advance_index(MatchedInput, Index) when is_integer(MatchedInput) -> % single characters
  {{line, Line}, {column, Col}} = Index,
  case MatchedInput of
    $\n -> {{line, Line+1}, {column, 1}};
    _ -> {{line, Line}, {column, Col+1}}
  end.
