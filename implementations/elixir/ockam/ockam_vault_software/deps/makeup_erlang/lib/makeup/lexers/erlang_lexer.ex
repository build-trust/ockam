defmodule Makeup.Lexers.ErlangLexer do
  @moduledoc """
  A `Makeup` lexer for the `Erlang` language.
  """

  @behaviour Makeup.Lexer

  import NimbleParsec
  import Makeup.Lexer.Combinators
  import Makeup.Lexer.Groups

  ###################################################################
  # Step #1: tokenize the input (into a list of tokens)
  ###################################################################

  whitespace = ascii_string([?\s, ?\f, ?\n], min: 1) |> token(:whitespace)

  # This is the combinator that ensures that the lexer will never reject a file
  # because of invalid input syntax
  any_char = utf8_char([]) |> token(:error)

  comment =
    ascii_char([?%])
    |> optional(utf8_string([not: ?\n], min: 1))
    |> token(:comment_single)

  hashbang =
    string("\n#!")
    |> utf8_string([not: ?\n], min: 1)
    |> string("\n")
    |> token(:comment_hashbang)

  escape_octal = ascii_string([?0..?7], min: 1, max: 3)

  escape_char = ascii_char([?\b, ?\d, ?\e, ?\f, ?\n, ?\r, ?\s, ?\t, ?\v, ?\', ?\", ?\\])

  escape_hex =
    choice([
      string("x") |> ascii_string([?0..?9, ?a..?f, ?A..?F], 2),
      string("x{") |> ascii_string([?0..?9, ?a..?f, ?A..?F], min: 1) |> string("}")
    ])

  escape_ctrl = string("^") |> ascii_char([?a..?z, ?A..?Z])

  escape =
    choice([
      escape_char,
      escape_octal,
      escape_hex,
      escape_ctrl
    ])

  numeric_base =
    choice([
      ascii_char([?1..?2]) |> ascii_char([?0..?9]),
      string("3") |> ascii_char([?0..?6]),
      ascii_char([?2..?9])
    ])

  # Numbers
  digits = ascii_string([?0..?9], min: 1)

  number_integer =
    optional(ascii_char([?+, ?-]))
    |> concat(digits)
    |> token(:number_integer)

  number_integer_in_weird_base =
    optional(ascii_char([?+, ?-]))
    |> concat(numeric_base)
    |> string("#")
    |> ascii_string([?0..?9, ?a..?z, ?A..?Z], min: 1)
    |> token(:number_integer)

  # Floating point numbers
  float_scientific_notation_part =
    ascii_string([?e, ?E], 1)
    |> optional(string("-"))
    |> concat(digits)

  number_float =
    optional(ascii_char([?+, ?-]))
    |> concat(digits)
    |> string(".")
    |> concat(digits)
    |> optional(float_scientific_notation_part)
    |> token(:number_float)

  variable_name =
    ascii_string([?A..?Z, ?_], 1)
    |> optional(ascii_string([?a..?z, ?_, ?0..?9, ?A..?Z], min: 1))

  simple_atom_name =
    ascii_string([?a..?z], 1)
    |> optional(ascii_string([?a..?z, ?_, ?0..?9, ?A..?Z], min: 1))
    |> reduce({Enum, :join, []})

  single_quote_escape = string("\\'")

  quoted_atom_name_middle =
    lookahead_not(string("'"))
    |> choice([
      single_quote_escape,
      utf8_string([not: ?\n, not: ?', not: ?\\], min: 1),
      escape
    ])

  quoted_atom_name =
    string("'")
    |> repeat(quoted_atom_name_middle)
    |> concat(string("'"))

  atom_name =
    choice([
      simple_atom_name,
      quoted_atom_name
    ])

  atom = token(atom_name, :string_symbol)

  namespace =
    token(atom_name, :name_class)
    |> concat(token(":", :punctuation))

  function =
    atom_name
    |> lexeme()
    |> token(:name_function)
    |> concat(optional(whitespace))
    |> concat(token("(", :punctuation))

  # Can also be a function name
  variable =
    variable_name
    # Check if you need to use the lexeme parser
    # (i.e. if you need the token value to be a string)
    # If not, just delete the lexeme parser
    |> lexeme()
    |> token(:name)

  macro_name = choice([variable_name, atom_name])

  macro =
    string("?")
    |> concat(macro_name)
    |> token(:name_constant)

  label =
    string("#")
    |> concat(atom_name)
    |> optional(string(".") |> concat(atom_name))
    |> token(:name_label)

  character =
    string("$")
    |> choice([
      escape,
      string("\\") |> ascii_char([?\s, ?%]),
      ascii_char(not: ?\\)
    ])
    |> token(:string_char)

  string_interpol =
    string("~")
    |> optional(ascii_string([?0..?9, ?., ?*], min: 1))
    |> ascii_char(to_charlist("~#+BPWXb-ginpswx"))
    |> token(:string_interpol)

  escape_double_quote = string(~s/\\"/)
  erlang_string = string_like(~s/"/, ~s/"/, [escape_double_quote, string_interpol], :string)

  # Combinators that highlight expressions surrounded by a pair of delimiters.
  punctuation =
    word_from_list([","] ++ ~w[\[ \] : _ @ \" . \#{ { } ( ) | ; => := << >> || -> \#], :punctuation)

  tuple = many_surrounded_by(parsec(:root_element), "{", "}")

  syntax_operators =
    word_from_list(~W[+ - +? ++ = == -- * / < > /= =:= =/= =< >= ==? <- ! ? ?!], :operator)

  record =
    token(string("#"), :operator)
    |> concat(atom)
    |> choice([
      token("{", :punctuation),
      token(".", :punctuation)
    ])

  # We need to match on the new line here as to not tokenize a function call as a module attribute.
  # Without the newline matching, the expression `a(X) - b(Y)` would tokenize
  # `b(Y)` as a module attribute definition instead of a function call.
  module_attribute =
    token("\n", :whitespace)
    |> optional(whitespace)
    |> concat(token("-", :punctuation))
    |> optional(whitespace)
    |> concat(atom_name |> token(:name_attribute))
    |> optional(whitespace)
    |> optional(token("(", :punctuation))

  function_arity =
    atom
    |> concat(token("/", :punctuation))
    |> concat(number_integer)

  # Tag the tokens with the language name.
  # This makes it easier to postprocess files with multiple languages.
  @doc false
  def __as_erlang_language__({ttype, meta, value}) do
    {ttype, Map.put(meta, :language, :erlang), value}
  end

  root_element_combinator =
    choice([
      module_attribute,
      hashbang,
      whitespace,
      comment,
      erlang_string,
      record,
      punctuation,
      # `tuple` might be unnecessary
      tuple,
      syntax_operators,
      # Numbers
      number_integer_in_weird_base,
      number_float,
      number_integer,
      # Variables
      variable,
      namespace,
      function_arity,
      function,
      atom,
      macro,
      character,
      label,
      # If we can't parse any of the above, we highlight the next character as an error
      # and proceed from there.
      # A lexer should always consume any string given as input.
      any_char
    ])

  ##############################################################################
  # Semi-public API: these two functions can be used by someone who wants to
  # embed this lexer into another lexer, but other than that, they are not
  # meant to be used by end-users
  ##############################################################################

  @inline Application.get_env(:makeup_erlang, :inline, false)

  @impl Makeup.Lexer
  defparsec(
    :root_element,
    root_element_combinator |> map({__MODULE__, :__as_erlang_language__, []}),
    inline: @inline
  )

  @impl Makeup.Lexer
  defparsec(
    :root,
    repeat(parsec(:root_element)),
    inline: @inline
  )

  ###################################################################
  # Step #2: postprocess the list of tokens
  ###################################################################

  @keywords ~W[after begin case catch cond end fun if let of query receive try when]

  @builtins ~W[
    abs append_element apply atom_to_list binary_to_list bitstring_to_list
    binary_to_term bit_size bump_reductions byte_size cancel_timer
    check_process_code delete_module demonitor disconnect_node display
    element erase exit float float_to_list fun_info fun_to_list
    function_exported garbage_collect get get_keys group_leader hash
    hd integer_to_list iolist_to_binary iolist_size is_atom is_binary
    is_bitstring is_boolean is_builtin is_float is_function is_integer
    is_list is_number is_pid is_port is_process_alive is_record is_reference
    is_tuple length link list_to_atom list_to_binary list_to_bitstring
    list_to_existing_atom list_to_float list_to_integer list_to_pid
    list_to_tuple load_module localtime_to_universaltime make_tuple
    md5 md5_final md5_update memory module_loaded monitor monitor_node
    node nodes open_port phash phash2 pid_to_list port_close port_command
    port_connect port_control port_call port_info port_to_list
    process_display process_flag process_info purge_module put read_timer
    ref_to_list register resume_processround send send_after send_nosuspend
    set_cookie setelement size spawn spawn_link spawn_monitor spawn_opt
    split_binary start_timer statistics suspend_process system_flag
    system_info system_monitor system_profile term_to_binary tl trace
    trace_delivered trace_info trace_pattern trunc tuple_size tuple_to_list
    universaltime_to_localtime unlink unregister whereis
  ]

  @word_operators ~W[and andalso band bnot bor bsl bsr bxor div not or orelse rem xor]

  defp postprocess_helper([{:string_symbol, meta, value} | tokens]) when value in @keywords,
    do: [{:keyword, meta, value} | postprocess_helper(tokens)]

  defp postprocess_helper([{:string_symbol, meta, value} | tokens]) when value in @builtins,
    do: [{:name_builtin, meta, value} | postprocess_helper(tokens)]

  defp postprocess_helper([{:string_symbol, meta, value} | tokens]) when value in @word_operators,
    do: [{:operator_word, meta, value} | postprocess_helper(tokens)]

  defp postprocess_helper([token | tokens]), do: [token | postprocess_helper(tokens)]

  defp postprocess_helper([]), do: []

  # By default, return the list of tokens unchanged
  @impl Makeup.Lexer
  def postprocess(tokens, _opts \\ []), do: postprocess_helper(tokens)

  #######################################################################
  # Step #3: highlight matching delimiters
  # By default, this includes delimiters that are used in many languages,
  # but feel free to delete these or add more.
  #######################################################################

  @impl Makeup.Lexer
  defgroupmatcher(:match_groups,
    parentheses: [
      open: [[{:punctuation, %{language: :erlang}, "("}]],
      close: [[{:punctuation, %{language: :erlang}, ")"}]]
    ],
    list: [
      open: [
        [{:punctuation, %{language: :erlang}, "["}]
      ],
      close: [
        [{:punctuation, %{language: :erlang}, "]"}]
      ]
    ],
    tuple: [
      open: [
        [{:punctuation, %{language: :erlang}, "{"}]
      ],
      close: [
        [{:punctuation, %{language: :erlang}, "}"}]
      ]
    ],
    map: [
      open: [
        [{:punctuation, %{language: :erlang}, "\#{"}]
      ],
      close: [
        [{:punctuation, %{language: :erlang}, "}"}]
      ]
    ]
  )

  defp remove_initial_newline([{ttype, meta, text} | tokens]) do
    case to_string(text) do
      "\n" -> tokens
      "\n" <> rest -> [{ttype, meta, rest} | tokens]
    end
  end

  # Finally, the public API for the lexer
  @impl Makeup.Lexer
  def lex(text, opts \\ []) do
    group_prefix = Keyword.get(opts, :group_prefix, random_prefix(10))
    {:ok, tokens, "", _, _, _} = root("\n" <> text)

    tokens
    |> remove_initial_newline()
    |> postprocess()
    |> match_groups(group_prefix)
  end
end
