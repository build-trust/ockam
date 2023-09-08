defmodule NimbleParsec.Compiler do
  @moduledoc false
  @arity 6

  @doc """
  Returns a parsec entrypoint named `name`.
  """
  def entry_point(name) do
    doc = """
    Parses the given `binary` as #{name}.

    Returns `{:ok, [token], rest, context, position, byte_offset}` or
    `{:error, reason, rest, context, line, byte_offset}` where `position`
    describes the location of the #{name} (start position) as `{line, column_on_line}`.

    ## Options

      * `:byte_offset` - the byte offset for the whole binary, defaults to 0
      * `:line` - the line and the byte offset into that line, defaults to `{1, byte_offset}`
      * `:context` - the initial context value. It will be converted to a map
    """

    spec =
      quote do
        unquote(name)(binary, keyword) ::
          {:ok, [term], rest, context, line, byte_offset}
          | {:error, reason, rest, context, line, byte_offset}
        when line: {pos_integer, byte_offset},
             byte_offset: pos_integer,
             rest: binary,
             reason: String.t(),
             context: map()
      end

    args = quote(do: [binary, opts \\ []])
    guards = quote(do: is_binary(binary))

    body =
      quote do
        context = Map.new(Keyword.get(opts, :context, []))
        byte_offset = Keyword.get(opts, :byte_offset, 0)

        line =
          case Keyword.get(opts, :line, 1) do
            {_, _} = line -> line
            line -> {line, byte_offset}
          end

        case unquote(:"#{name}__0")(binary, [], [], context, line, byte_offset) do
          {:ok, acc, rest, context, line, offset} ->
            {:ok, :lists.reverse(acc), rest, context, line, offset}

          {:error, _, _, _, _, _} = error ->
            error
        end
      end

    {doc, spec, {name, args, guards, body}}
  end

  @doc """
  Compiles the given combinators into multiple definitions.
  """
  def compile(name, [], _opts) do
    raise ArgumentError, "cannot compile #{inspect(name)} with an empty parser combinator"
  end

  def compile(name, combinators, opts) when is_list(combinators) do
    inline? = Keyword.get(opts, :inline, false)
    {defs, inline} = compile(name, combinators)

    if inline? do
      {defs, inline}
    else
      {defs, []}
    end
  end

  defp compile(name, combinators) do
    config = %{
      acc_depth: 0,
      catch_all: nil,
      labels: [],
      name: name,
      replace: false
    }

    {next, step} = build_next(0, config)

    {defs, inline, last, _step} =
      combinators
      |> Enum.reverse()
      |> compile([], [], next, step, config)

    {Enum.reverse([build_ok(last) | defs]), [{last, @arity} | inline]}
  end

  defp compile([], defs, inline, current, step, _config) do
    {defs, inline, current, step}
  end

  defp compile([{:update, key, fun} | combinators], defs, inline, current, step, config) do
    compile(combinators, defs, inline, current, step, Map.update!(config, key, fun))
  end

  defp compile(combinators, defs, inline, current, step, config) do
    {next_combinators, used_combinators, {new_defs, new_inline, next, step, catch_all}} =
      case take_bound_combinators(combinators) do
        {[combinator | combinators], [], [], [], [], _metadata} ->
          case combinator do
            {:label, label_combinators, label} ->
              pre_combinators = [{:update, :labels, &[label | &1]} | label_combinators]
              pos_combinators = [{:update, :labels, &tl(&1)} | combinators]

              {pre_combinators ++ pos_combinators, [combinator],
               {[], [], current, step, :catch_none}}

            _ ->
              {combinators, [combinator],
               compile_unbound_combinator(combinator, current, step, config)}
          end

        {combinators, inputs, guards, outputs, acc, metadata} ->
          {combinators, Enum.reverse(acc),
           compile_bound_combinator(inputs, guards, outputs, metadata, current, step, config)}
      end

    catch_all_defs =
      case catch_all do
        :catch_all -> [build_catch_all(:positive, current, used_combinators, config)]
        :catch_none -> []
      end

    defs = catch_all_defs ++ Enum.reverse(new_defs) ++ defs
    compile(next_combinators, defs, new_inline ++ inline, next, step, config)
  end

  ## Unbound combinators

  defp compile_unbound_combinator({:parsec, parsec}, current, step, config) do
    {next, step} = build_next(step, config)
    head = quote(do: [rest, acc, stack, context, line, offset])

    catch_all =
      case config do
        %{catch_all: nil} ->
          quote(do: error)

        %{catch_all: catch_all, acc_depth: n} ->
          {_, _, _, body} = build_proxy_to(current, catch_all, n)
          body
      end

    call =
      case parsec do
        {mod, fun} ->
          quote do
            unquote(mod).unquote(:"#{fun}__0")(rest, acc, [], context, line, offset)
          end

        fun ->
          quote do
            unquote(:"#{fun}__0")(rest, acc, [], context, line, offset)
          end
      end

    body =
      quote do
        case unquote(call) do
          {:ok, acc, rest, context, line, offset} ->
            unquote(next)(rest, acc, stack, context, line, offset)

          {:error, _, _, _, _, _} = error ->
            unquote(catch_all)
        end
      end

    def = {current, head, true, body}
    {[def], [{current, @arity}], next, step, :catch_none}
  end

  defp compile_unbound_combinator({:lookahead, combinators, kind}, current, step, config) do
    choices = extract_choices_from_lookahead(combinators)

    if Enum.all?(choices, &all_bound_combinators?/1) do
      {next, step} = build_next(step, config)
      args = quote(do: [rest, acc, stack, context, line, offset])
      success_body = {next, [], args}
      {_, _, _, failure_body} = build_catch_all(kind, current, combinators, config)

      {success_body, failure_body} =
        if kind == :positive, do: {success_body, failure_body}, else: {failure_body, success_body}

      defs =
        for choice <- choices do
          {[], inputs, guards, _, _, metadata} = take_bound_combinators(choice)
          {bin, _} = compile_bound_bin_pattern(inputs, metadata, quote(do: _))
          head = quote(do: [unquote(bin) = rest, acc, stack, context, line, offset])
          guards = guards_list_to_quoted(guards)
          {current, head, guards, success_body}
        end

      defs = if [] in choices, do: defs, else: defs ++ [{current, args, true, failure_body}]
      {defs, [], next, step, :catch_none}
    else
      compile_unbound_lookahead(combinators, kind, current, step, config)
    end
  end

  defp compile_unbound_combinator(
         {:traverse, combinators, kind, traversal},
         current,
         step,
         config
       ) do
    fun = &traverse(traversal, &1, &2, &3, &4, &5, &6, config)
    config = if kind == :constant, do: put_in(config.replace, true), else: config
    compile_unbound_traverse(combinators, kind, current, step, config, fun)
  end

  defp compile_unbound_combinator({:times, combinators, 0, count}, current, step, config) do
    if all_no_context_combinators?(combinators) do
      compile_bound_times(combinators, count, current, step, config)
    else
      compile_unbound_times(combinators, count, current, step, config)
    end
  end

  defp compile_unbound_combinator({:repeat, combinators, while}, current, step, config) do
    {failure, step} = build_next(step, config)
    config = %{config | catch_all: failure, acc_depth: 0}

    if all_no_context_combinators?(combinators) do
      compile_bound_repeat(combinators, while, current, failure, step, config)
    else
      compile_unbound_repeat(combinators, while, current, failure, step, config)
    end
  end

  defp compile_unbound_combinator({:eventually, combinators}, current, step, config) do
    compile_eventually(combinators, current, step, config)
  end

  defp compile_unbound_combinator({:choice, choices} = combinator, current, step, config) do
    config =
      update_in(config.labels, fn
        [] -> [label(combinator)]
        other -> other
      end)

    if Enum.all?(choices, &all_bound_combinators?/1) do
      compile_bound_choice(choices, current, step, config)
    else
      compile_unbound_choice(choices, current, step, config)
    end
  end

  ## Lookahead

  defp extract_choices_from_lookahead([{:choice, choices}]), do: choices
  defp extract_choices_from_lookahead(other), do: [other]

  defp compile_unbound_lookahead(combinators, kind, current, step, config) do
    {_, _, _, catch_all} = build_catch_all(kind, current, combinators, config)

    {next, step} = build_next(step, config)
    head = quote(do: [rest, acc, stack, context, line, offset])

    args =
      quote(do: [rest, [], [{rest, acc, context, line, offset} | stack], context, line, offset])

    body = {next, [], args}
    entry_point = {current, head, true, body}

    {failure, step} = build_next(step, config)
    config = %{config | catch_all: failure, acc_depth: 0}
    {defs, inline, success, step} = compile(combinators, [entry_point], [], next, step, config)

    {next, step} = build_next(step, config)
    head = quote(do: [_, _, [{rest, acc, context, line, offset} | stack], _, _, _])
    args = quote(do: [rest, acc, stack, context, line, offset])
    body = {next, [], args}

    success_failure =
      if kind == :positive do
        [{success, head, true, body}, {failure, head, true, catch_all}]
      else
        [{failure, head, true, body}, {success, head, true, catch_all}]
      end

    inline = [{current, @arity}, {success, @arity}, {failure, @arity} | inline]
    {Enum.reverse(success_failure ++ defs), inline, next, step, :catch_none}
  end

  ## Traverse

  defp compile_unbound_traverse([], _kind, current, step, config, fun) do
    {next, step} = build_next(step, config)
    head = quote(do: [rest, acc, stack, context, line, offset])
    [rest, _, _, context, line, offset] = head

    body = fun.(next, rest, [], context, line, offset)
    def = {current, head, true, body}
    {[def], [{current, @arity}], next, step, :catch_none}
  end

  defp compile_unbound_traverse(combinators, kind, current, step, config, fun) do
    {next, step} = build_next(step, config)
    head = quote(do: [rest, acc, stack, context, line, offset])

    args =
      if kind == :pre do
        quote(do: [rest, [], [{acc, line, offset} | stack], context, line, offset])
      else
        quote(do: [rest, [], [acc | stack], context, line, offset])
      end

    body = {next, [], args}
    entry_point = {current, head, true, body}

    config = update_in(config.acc_depth, &(&1 + 1))
    {defs, inline, last, step} = compile(combinators, [entry_point], [], next, step, config)

    # Now we need to traverse the accumulator with the user code and
    # concatenate with the previous accumulator at the top of the stack.
    {next, step} = build_next(step, config)

    {head, {traverse_line, traverse_offset}} =
      if kind == :pre do
        quote do
          {[rest, user_acc, [{acc, stack_line, stack_offset} | stack], context, line, offset],
           {stack_line, stack_offset}}
        end
      else
        quote do
          {[rest, user_acc, [acc | stack], context, line, offset], {line, offset}}
        end
      end

    [rest, user_acc, _, context | _] = head
    body = fun.(next, rest, user_acc, context, traverse_line, traverse_offset)
    last_def = {last, head, true, body}

    inline = [{current, @arity}, {last, @arity} | inline]
    {Enum.reverse([last_def | defs]), inline, next, step, :catch_none}
  end

  defp traverse(_traversal, next, _, user_acc, _, _, _, %{replace: true}) do
    quote do
      _ = unquote(user_acc)
      unquote(next)(rest, acc, stack, context, line, offset)
    end
  end

  defp traverse(traversal, next, rest, user_acc, context, line, offset, _) do
    case apply_traverse(traversal, rest, user_acc, context, line, offset) do
      {expanded_acc, ^context} when user_acc != :error ->
        quote do
          _ = unquote(user_acc)
          unquote(next)(rest, unquote(expanded_acc) ++ acc, stack, context, line, offset)
        end

      quoted ->
        quote do
          case unquote(quoted) do
            {user_acc, context} when is_list(user_acc) ->
              unquote(next)(rest, user_acc ++ acc, stack, context, line, offset)

            {:error, reason} ->
              {:error, reason, rest, context, line, offset}
          end
        end
    end
  end

  defp apply_traverse(mfargs, rest, acc, context, line, offset) do
    apply_traverse(Enum.reverse(mfargs), rest, {acc, context}, line, offset)
  end

  defp apply_traverse([mfargs | tail], rest, {acc, context}, line, offset) when acc != :error do
    acc_context = apply_mfa(mfargs, [rest, acc, context, line, offset])
    apply_traverse(tail, rest, acc_context, line, offset)
  end

  defp apply_traverse([], _rest, acc_context, _line, _offset) do
    acc_context
  end

  defp apply_traverse(tail, rest, acc_context, line, offset) do
    pattern = quote(do: {acc, context} when is_list(acc))
    args = [rest, quote(do: acc), quote(do: context), line, offset]

    entries =
      Enum.map(tail, fn mfargs ->
        quote(do: unquote(pattern) <- unquote(apply_mfa(mfargs, args)))
      end)

    quote do
      with unquote(pattern) <- unquote(acc_context), unquote_splicing(entries) do
        {acc, context}
      end
    end
  end

  ## Eventually

  defp compile_eventually(combinators, current, step, config) do
    {failure, step} = build_next(step, config)
    failure_def = build_eventually_next_def(current, failure)
    catch_all_def = build_catch_all(:positive, failure, combinators, config)

    config = %{config | catch_all: failure, acc_depth: 0}
    {defs, inline, success, step} = compile(combinators, [], [], current, step, config)

    defs = Enum.reverse(defs, [failure_def, catch_all_def])
    {defs, [{failure, @arity} | inline], success, step, :catch_none}
  end

  defp build_eventually_next_def(current, failure) do
    head = quote(do: [<<byte, rest::binary>>, acc, stack, context, line, offset])
    offset = add_offset(quote(do: offset), 1)
    line = add_line(quote(do: line), offset, quote(do: byte))
    body = {current, [], quote(do: [rest, acc, stack, context]) ++ [line, offset]}
    {failure, head, true, body}
  end

  ## Repeat

  defp compile_bound_repeat(combinators, while, current, failure, step, config) do
    {defs, recur, next, step} =
      case apply_mfa(while, quote(do: [rest, context, line, offset])) do
        {:cont, quote(do: context)} ->
          {[], current, current, step}

        quoted ->
          {next, step} = build_next(step, config)
          head = args = quote(do: [rest, acc, stack, context, line, offset])
          body = repeat_while(quoted, next, args, failure, args)
          {[{current, head, true, body}], current, next, step}
      end

    {defs, inline, success, step} = compile(combinators, defs, [], next, step, config)
    def = build_proxy_to(success, recur, 0)
    {Enum.reverse([def | defs]), [{success, @arity} | inline], failure, step, :catch_none}
  end

  defp compile_unbound_repeat(combinators, while, current, failure, step, config) do
    {recur, step} = build_next(step, config)
    {defs, inline, success, step} = compile(combinators, [], [], recur, step, config)

    {next, step} = build_next(step, config)
    head = quote(do: [_, _, [{rest, acc, context, line, offset} | stack], _, _, _])
    args = quote(do: [rest, acc, stack, context, line, offset])
    body = {next, [], args}
    failure_def = {failure, head, true, body}

    while = apply_mfa(while, quote(do: [rest, context, line, offset]))
    cont = quote(do: {rest, acc, context, line, offset})

    head =
      quote do
        [inner_rest, inner_acc, [unquote(cont) | stack], inner_context, inner_line, inner_offset]
      end

    cont = quote(do: {inner_rest, inner_acc ++ acc, inner_context, inner_line, inner_offset})

    true_args =
      quote do
        [inner_rest, [], [unquote(cont) | stack], inner_context, inner_line, inner_offset]
      end

    false_args = quote(do: [rest, acc, stack, context, line, offset])

    # We need to do this dance because of unused variables
    body =
      case compile_time_repeat_while(while) do
        :cont ->
          quote do
            _ = {rest, acc, context, line, offset}
            unquote({recur, [], true_args})
          end

        :halt ->
          quote do
            _ = {inner_rest, inner_acc, inner_context, inner_line, inner_offset}
            unquote({next, [], false_args})
          end

        :none ->
          repeat_while(while, recur, true_args, next, false_args)
      end

    success_def = {success, head, true, body}
    head = quote(do: [rest, acc, stack, context, line, offset])

    true_args =
      quote do
        [rest, [], [{rest, acc, context, line, offset} | stack], context, line, offset]
      end

    false_args = quote(do: [rest, acc, stack, context, line, offset])
    body = repeat_while(while, recur, true_args, next, false_args)
    current_def = {current, head, true, body}

    defs = [current_def | Enum.reverse([success_def, failure_def | defs])]
    inline = [{current, @arity}, {success, @arity}, {failure, @arity} | inline]
    {defs, inline, next, step, :catch_none}
  end

  defp compile_time_repeat_while({:cont, quote(do: context)}), do: :cont
  defp compile_time_repeat_while({:halt, quote(do: context)}), do: :halt
  defp compile_time_repeat_while(_), do: :none

  defp repeat_while(quoted, true_name, true_args, false_name, false_args) do
    case compile_time_repeat_while(quoted) do
      :cont ->
        {true_name, [], true_args}

      :halt ->
        {false_name, [], false_args}

      :none ->
        quote do
          case unquote(quoted) do
            {:cont, context} -> unquote({true_name, [], true_args})
            {:halt, context} -> unquote({false_name, [], false_args})
          end
        end
    end
  end

  ## Repeat up to

  defp compile_bound_times(combinators, count, current, step, config) do
    {failure, step} = build_next(step, config)
    {recur, step} = build_next(step, config)

    head = quote(do: [rest, acc, stack, context, line, offset])
    args = quote(do: [rest, acc, [unquote(count) | stack], context, line, offset])
    body = {recur, [], args}
    current_def = {current, head, true, body}

    config = %{config | catch_all: failure, acc_depth: 0}
    {defs, inline, success, step} = compile(combinators, [current_def], [], recur, step, config)

    {next, step} = build_next(step, config)
    head = quote(do: [rest, acc, [1 | stack], context, line, offset])
    args = quote(do: [rest, acc, stack, context, line, offset])
    body = {next, [], args}
    success_def0 = {success, head, true, body}

    head = quote(do: [rest, acc, [count | stack], context, line, offset])
    args = quote(do: [rest, acc, [count - 1 | stack], context, line, offset])
    body = {recur, [], args}
    success_def1 = {success, head, true, body}

    head = quote(do: [rest, acc, [_ | stack], context, line, offset])
    args = quote(do: [rest, acc, stack, context, line, offset])
    body = {next, [], args}
    failure_def = {failure, head, true, body}

    defs = Enum.reverse([success_def1, success_def0, failure_def | defs])
    inline = [{current, @arity}, {success, @arity}, {failure, @arity} | inline]
    {defs, inline, next, step, :catch_none}
  end

  defp compile_unbound_times(combinators, count, current, step, config) do
    {failure, step} = build_next(step, config)
    {recur, step} = build_next(step, config)

    head = quote(do: [rest, acc, stack, context, line, offset])
    cont = quote(do: {unquote(count), rest, acc, context, line, offset})
    args = quote(do: [rest, [], [unquote(cont) | stack], context, line, offset])
    body = {recur, [], args}
    current_def = {current, head, true, body}

    config = %{config | catch_all: failure, acc_depth: 0}
    {defs, inline, success, step} = compile(combinators, [current_def], [], recur, step, config)

    {next, step} = build_next(step, config)
    head = quote(do: [rest, user_acc, [{1, _, acc, _, _, _} | stack], context, line, offset])
    args = quote(do: [rest, user_acc ++ acc, stack, context, line, offset])
    body = {next, [], args}
    success_def0 = {success, head, true, body}

    head = quote(do: [rest, user_acc, [{count, _, acc, _, _, _} | stack], context, line, offset])
    cont = quote(do: {count - 1, rest, user_acc ++ acc, context, line, offset})
    args = quote(do: [rest, [], [unquote(cont) | stack], context, line, offset])
    body = {recur, [], args}
    success_def1 = {success, head, true, body}

    head = quote(do: [_, _, [{_, rest, acc, context, line, offset} | stack], _, _, _])
    args = quote(do: [rest, acc, stack, context, line, offset])
    body = {next, [], args}
    failure_def = {failure, head, true, body}

    defs = Enum.reverse([success_def1, success_def0, failure_def | defs])
    inline = [{current, @arity}, {success, @arity}, {failure, @arity} | inline]
    {defs, inline, next, step, :catch_none}
  end

  ## Choice

  defp compile_bound_choice(choices, current, step, config) do
    {next_name, next_step} = build_next(step, config)

    defs =
      for choice <- choices do
        {[], inputs, guards, outputs, _, metadata} = take_bound_combinators(choice)

        {[def], [], ^next_name, ^next_step, _} =
          compile_bound_combinator(inputs, guards, outputs, metadata, current, step, config)

        def
      end

    catch_all = if [] in choices, do: :catch_none, else: :catch_all
    {defs, [], next_name, next_step, catch_all}
  end

  defp compile_unbound_choice(choices, current, step, config) do
    {done, step} = build_next(step, config)

    # We process choices in reverse order. The last order does not
    # have any fallback besides the requirement to drop the stack
    # this allows us to compose with repeat and traverse.
    config = update_in(config.acc_depth, &(&1 + 2))

    {first, defs, inline, step} =
      compile_unbound_choice(Enum.reverse(choices), [], [], :unused, step, done, config)

    head = quote(do: [rest, acc, stack, context, line, offset])
    cont = quote(do: {rest, context, line, offset})
    args = quote(do: [rest, [], [unquote(cont), acc | stack], context, line, offset])
    body = {first, [], args}
    def = {current, head, true, body}

    {[def | Enum.reverse(defs)], [{current, @arity} | inline], done, step, :catch_none}
  end

  defp compile_unbound_choice([], defs, inline, previous, step, _success, _config) do
    # Discard the last failure definition that won't be used.
    {previous, tl(defs), tl(inline), step - 1}
  end

  defp compile_unbound_choice([choice | choices], defs, inline, _previous, step, done, config) do
    {current, step} = build_next(step, config)
    {defs, inline, success, step} = compile(choice, defs, inline, current, step, config)

    head = quote(do: [rest, acc, [_, previous_acc | stack], context, line, offset])
    args = quote(do: [rest, acc ++ previous_acc, stack, context, line, offset])
    body = {done, [], args}
    success_def = {success, head, true, body}

    {failure, step} = build_next(step, config)
    head = quote(do: [_, _, [{rest, context, line, offset} | _] = stack, _, _, _])
    args = quote(do: [rest, [], stack, context, line, offset])
    body = {current, [], args}
    failure_def = {failure, head, true, body}

    defs = [failure_def, success_def | defs]
    inline = [{failure, @arity}, {success, @arity} | inline]
    config = %{config | catch_all: failure, acc_depth: 0}
    compile_unbound_choice(choices, defs, inline, current, step, done, config)
  end

  ## No context combinators

  # If a combinator does not need a context, i.e. it cannot abort
  # in the middle, then we can compile to an optimized version of
  # repeat and times.
  #
  # For example, a lookahead at the beginning doesn't need a context.
  # A choice that is bound doesn't need one either.
  defp all_no_context_combinators?([{:lookahead, look_combinators, _kind} | combinators]) do
    all_bound_combinators?(look_combinators) and
      all_no_context_combinators_next?(combinators)
  end

  defp all_no_context_combinators?(combinators) do
    all_no_context_combinators_next?(combinators)
  end

  defp all_no_context_combinators_next?([{:choice, choice_combinators, _kind} | combinators]) do
    all_bound_combinators?(choice_combinators) and
      all_no_context_combinators_next?(combinators)
  end

  defp all_no_context_combinators_next?(combinators) do
    all_bound_combinators?(combinators)
  end

  ## Bound combinators

  # A bound combinator is a combinator where the number of inputs, guards,
  # outputs, line and offset shifts are known at compilation time. We inline
  # those bound combinators into a single bitstring pattern for performance.
  # Currently error reporting will accuse the beginning of the bound combinator
  # in case of errors but such can be addressed if desired.

  defp compile_bound_combinator(inputs, guards, outputs, metadata, current, step, config) do
    %{line: line, offset: offset} = metadata
    {next, step} = build_next(step, config)
    {bin, rest} = compile_bound_bin_pattern(inputs, metadata, quote(do: rest))

    acc = if config.replace, do: quote(do: acc), else: quote(do: unquote(outputs) ++ acc)

    args =
      quote(do: [unquote(rest), unquote(acc), stack, context, unquote(line), unquote(offset)])

    head = quote(do: [unquote(bin), acc, stack, context, comb__line, comb__offset])
    body = {next, [], args}

    guards = guards_list_to_quoted(guards)
    def = {current, head, guards, body}
    {[def], [], next, step, :catch_all}
  end

  defp compile_bound_bin_pattern(inputs, %{eos: eos?}, var) do
    rest = if eos?, do: "", else: var
    bin = {:<<>>, [], inputs ++ [quote(do: unquote(rest) :: binary)]}
    {bin, rest}
  end

  defp all_bound_combinators?(combinators) do
    match?({[], _, _, _, _, _}, take_bound_combinators(combinators))
  end

  defp take_bound_combinators(combinators) do
    {line, offset} = line_offset_pair()
    metadata = %{eos: false, line: line, offset: offset, counter: 0}
    take_bound_combinators(combinators, [], [], [], [], metadata)
  end

  defp take_bound_combinators([:eos | combinators], inputs, guards, outputs, acc, metadata) do
    combinators = Enum.drop_while(combinators, &(&1 == :eos))
    {combinators, inputs, guards, outputs, [:eos | acc], %{metadata | eos: true}}
  end

  defp take_bound_combinators(combinators, inputs, guards, outputs, acc, metadata) do
    with [combinator | combinators] <- combinators,
         {:ok, new_inputs, new_guards, new_outputs, metadata} <-
           bound_combinator(combinator, metadata) do
      take_bound_combinators(
        combinators,
        inputs ++ new_inputs,
        guards ++ new_guards,
        merge_output(new_outputs, outputs),
        [combinator | acc],
        metadata
      )
    else
      _ ->
        {combinators, inputs, guards, outputs, acc, metadata}
    end
  end

  defp merge_output(left, right) when is_list(left) and is_list(right), do: left ++ right
  defp merge_output(left, right), do: quote(do: unquote(left) ++ unquote(right))

  defp bound_combinator({:string, string}, %{line: line, offset: offset} = metadata) do
    size = byte_size(string)

    line =
      case String.split(string, "\n") do
        [_] ->
          line

        [_ | _] = many ->
          last_size = many |> List.last() |> byte_size()
          line_offset = add_offset(offset, size - last_size)

          quote do
            {elem(unquote(line), 0) + unquote(length(many) - 1), unquote(line_offset)}
          end
      end

    offset = add_offset(offset, size)
    {:ok, [string], [], [string], %{metadata | line: line, offset: offset}}
  end

  defp bound_combinator({:bin_segment, inclusive, exclusive, modifiers}, metadata) do
    %{line: line, offset: offset, counter: counter} = metadata

    {var, counter} = build_var(counter)
    input = apply_bin_modifiers(var, modifiers)
    guards = compile_bin_ranges(var, inclusive, exclusive)

    offset =
      if :integer in modifiers do
        add_offset(offset, 1)
      else
        add_offset(offset, quote(do: byte_size(<<unquote(input)>>)))
      end

    line =
      if newline_allowed?(inclusive) and not newline_forbidden?(exclusive) do
        add_line(line, offset, var)
      else
        line
      end

    metadata = %{metadata | line: line, offset: offset, counter: counter}
    {:ok, [input], guards, [var], metadata}
  end

  defp bound_combinator({:label, combinators, _labels}, metadata) do
    case take_bound_combinators(combinators, [], [], [], [], metadata) do
      {[], inputs, guards, outputs, _, metadata} ->
        {:ok, inputs, guards, outputs, metadata}

      {_, _, _, _, _, _} ->
        :error
    end
  end

  defp bound_combinator({:traverse, combinators, kind, mfargs}, pre_metadata) do
    case take_bound_combinators(combinators, [], [], [], [], pre_metadata) do
      {[], inputs, guards, outputs, _, post_metadata} ->
        {rest, context} = quote(do: {rest, context})
        {traverse_line, traverse_offset} = pre_post_traverse(kind, pre_metadata, post_metadata)

        case apply_traverse(mfargs, rest, outputs, context, traverse_line, traverse_offset) do
          {outputs, ^context} when outputs != :error ->
            {:ok, inputs, guards, outputs, post_metadata}

          _ ->
            :error
        end

      {_, _, _, _, _, _} ->
        :error
    end
  end

  defp bound_combinator(_, _) do
    :error
  end

  ## Line and offset handling

  # For pre traversal returns the AST before, otherwise the AST after
  # for post. For constant, line/offset are never used.
  defp pre_post_traverse(:pre, %{line: line, offset: offset}, _), do: {line, offset}
  defp pre_post_traverse(_, _, %{line: line, offset: offset}), do: {line, offset}

  defp line_offset_pair() do
    quote(do: {comb__line, comb__offset})
  end

  defp add_offset({:+, _, [var, current]}, extra)
       when is_integer(current) and is_integer(extra) do
    {:+, [], [var, current + extra]}
  end

  defp add_offset(var, extra) do
    {:+, [], [var, extra]}
  end

  defp newline_allowed?([]), do: true

  defp newline_allowed?(ors) do
    Enum.any?(ors, fn
      _.._ = range -> ?\n in range
      codepoint -> ?\n === codepoint
    end)
  end

  defp newline_forbidden?([]), do: false

  defp newline_forbidden?(ands) do
    Enum.any?(ands, fn
      {:not, _.._ = range} -> ?\n in range
      {:not, codepoint} -> ?\n === codepoint
    end)
  end

  defp add_line(line, offset, var) do
    quote do
      line = unquote(line)

      case unquote(var) do
        ?\n -> {elem(line, 0) + 1, unquote(offset)}
        _ -> line
      end
    end
  end

  ## Label

  defp labels([]) do
    "nothing"
  end

  defp labels(combinators) do
    Enum.map_join(combinators, ", followed by ", &label/1)
  end

  defp label({:string, binary}) do
    "string #{inspect(binary)}"
  end

  defp label({:label, _combinator, label}) do
    label
  end

  defp label({:bin_segment, inclusive, exclusive, modifiers}) do
    {inclusive, printable?} = Enum.map_reduce(inclusive, true, &inspect_bin_range(&1, &2))

    {exclusive, printable?} =
      Enum.map_reduce(exclusive, printable?, &inspect_bin_range(elem(&1, 1), &2))

    prefix =
      cond do
        :integer in modifiers and not printable? -> "byte"
        :integer in modifiers -> "ASCII character"
        :utf8 in modifiers -> "utf8 codepoint"
        :utf16 in modifiers -> "utf16 codepoint"
        :utf32 in modifiers -> "utf32 codepoint"
      end

    prefix <> Enum.join([Enum.join(inclusive, " or") | exclusive], ", and not")
  end

  defp label(:eos) do
    "end of string"
  end

  defp label({:lookahead, combinators, _}) do
    labels(combinators)
  end

  defp label({:repeat, combinators, _}) do
    labels(combinators)
  end

  defp label({:eventually, combinators}) do
    labels(combinators) <> " eventually"
  end

  defp label({:times, combinators, _, _}) do
    labels(combinators)
  end

  defp label({:choice, choices}) do
    Enum.map_join(choices, " or ", &labels/1)
  end

  defp label({:traverse, combinators, _, _}) do
    labels(combinators)
  end

  defp label({:parsec, {_module, function}}) do
    Atom.to_string(function)
  end

  defp label({:parsec, name}) do
    Atom.to_string(name)
  end

  ## Bin segments

  defp compile_bin_ranges(var, ors, ands) do
    ands = Enum.map(ands, &bin_range_to_guard(var, &1))

    if ors == [] do
      ands
    else
      ors =
        ors
        |> Enum.map(&bin_range_to_guard(var, &1))
        |> Enum.reduce(&{:or, [], [&2, &1]})

      [ors | ands]
    end
  end

  defp bin_range_to_guard(var, range) do
    case range do
      min..max when min < max ->
        quote(do: unquote(var) >= unquote(min) and unquote(var) <= unquote(max))

      min..max when min > max ->
        quote(do: unquote(var) >= unquote(max) and unquote(var) <= unquote(min))

      min..min ->
        quote(do: unquote(var) === unquote(min))

      min when is_integer(min) ->
        quote(do: unquote(var) === unquote(min))

      {:not, min..max} when min < max ->
        quote(do: unquote(var) < unquote(min) or unquote(var) > unquote(max))

      {:not, min..max} when min > max ->
        quote(do: unquote(var) < unquote(max) or unquote(var) > unquote(min))

      {:not, min..min} ->
        quote(do: unquote(var) !== unquote(min))

      {:not, min} when is_integer(min) ->
        quote(do: unquote(var) !== unquote(min))
    end
  end

  defp inspect_bin_range(min..max, printable?) do
    {" in the range #{inspect_char(min)} to #{inspect_char(max)}",
     printable? and printable?(min) and printable?(max)}
  end

  defp inspect_bin_range(min, printable?) do
    {" equal to #{inspect_char(min)}", printable? and printable?(min)}
  end

  defp printable?(codepoint), do: List.ascii_printable?([codepoint])
  defp inspect_char(codepoint), do: inspect([codepoint], charlists: :as_charlists)

  defp apply_bin_modifiers(expr, modifiers) do
    case modifiers do
      [] ->
        expr

      _ ->
        modifiers = Enum.map(modifiers, &Macro.var(&1, __MODULE__))
        {:"::", [], [expr, Enum.reduce(modifiers, &{:-, [], [&2, &1]})]}
    end
  end

  ## Helpers

  defp apply_mfa({mod, fun, args}, extra) do
    apply(mod, fun, extra ++ args)
  end

  defp guards_list_to_quoted([]), do: true
  defp guards_list_to_quoted(guards), do: Enum.reduce(guards, &{:and, [], [&2, &1]})

  defp build_var(counter) do
    {{:"x#{counter}", [], __MODULE__}, counter + 1}
  end

  defp build_next(step, %{name: name}) do
    {:"#{name}__#{step}", step + 1}
  end

  defp build_ok(current) do
    head = quote(do: [rest, acc, _stack, context, line, offset])
    body = quote(do: {:ok, acc, rest, context, line, offset})
    {current, head, true, body}
  end

  defp build_catch_all(kind, name, combinators, %{catch_all: nil, labels: labels}) do
    reason = error_reason(combinators, labels)
    reason = if kind == :positive, do: "expected " <> reason, else: "did not expect " <> reason
    args = quote(do: [rest, _acc, _stack, context, line, offset])
    body = quote(do: {:error, unquote(reason), rest, context, line, offset})
    {name, args, true, body}
  end

  defp build_catch_all(_kind, name, _combinators, %{catch_all: next, acc_depth: n}) do
    build_proxy_to(name, next, n)
  end

  defp build_acc_depth(1, acc, stack), do: [{:|, [], [acc, stack]}]
  defp build_acc_depth(n, acc, stack), do: [quote(do: _) | build_acc_depth(n - 1, acc, stack)]

  defp build_proxy_to(name, next, 0) do
    args = quote(do: [rest, acc, stack, context, line, offset])
    body = {next, [], args}
    {name, args, true, body}
  end

  defp build_proxy_to(name, next, n) do
    args = quote(do: [rest, _acc, stack, context, line, offset])
    {acc, stack} = quote(do: {acc, stack})

    body =
      quote do
        unquote(build_acc_depth(n, acc, stack)) = stack
        unquote(next)(rest, acc, stack, context, line, offset)
      end

    {name, args, true, body}
  end

  defp error_reason(combinators, []) do
    labels(combinators)
  end

  defp error_reason(_combinators, [head]) do
    head
  end

  defp error_reason(_combinators, [head | tail]) do
    "#{head} while processing #{Enum.join(tail, " inside ")}"
  end
end
