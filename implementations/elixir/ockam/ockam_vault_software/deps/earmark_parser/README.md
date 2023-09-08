
# EarmarkParser A Pure Elixir Markdown Parser (split from Earmark)

[![CI](https://github.com/robertdober/earmark_parser/workflows/CI/badge.svg)](https://github.com/robertdober/earmark_parser/actions)
[![Coverage Status](https://coveralls.io/repos/github/RobertDober/earmark_parser/badge.svg?branch=master)](https://coveralls.io/github/RobertDober/earmark_parser?branch=master)
[![Hex.pm](https://img.shields.io/hexpm/v/earmark_parser.svg)](https://hex.pm/packages/earmark_parser)
[![Hex.pm](https://img.shields.io/hexpm/dw/earmark_parser.svg)](https://hex.pm/packages/earmark_parser)
[![Hex.pm](https://img.shields.io/hexpm/dt/earmark_parser.svg)](https://hex.pm/packages/earmark_parser)


## Table Of Contents

- [Table Of Contents](#table-of-contents)
- [Usage](#usage)
  - [EarmarkParser](#earmarkparser)
  - [API](#api)
    - [EarmarkParser.as_ast](#earmarkparseras_ast)
    - [Options](#options)
- [Supports](#supports)
- [Extensions](#extensions)
  - [Links](#links)
    - [Links supported by default](#links-supported-by-default)
    - [Autolinks](#autolinks)
    - [Additional link parsing via options](#additional-link-parsing-via-options)
    - [Pure links](#pure-links)
    - [Wikilinks...](#wikilinks)
  - [Github Flavored Markdown](#github-flavored-markdown)
    - [Strike Through](#strike-through)
    - [Syntax Highlighting](#syntax-highlighting)
    - [Tables](#tables)
    - [HTML Blocks](#html-blocks)
    - [HTML Comments](#html-comments)
  - [Adding Attributes with the IAL extension](#adding-attributes-with-the-ial-extension)
    - [To block elements](#to-block-elements)
    - [To links or images](#to-links-or-images)
- [Limitations](#limitations)
- [Timeouts](#timeouts)
- [Annotations](#annotations)
  - [Annotated Paragraphs](#annotated-paragraphs)
  - [Annotated HTML elements](#annotated-html-elements)
  - [Commenting your Markdown](#commenting-your-markdown)
  - [EarmarkParser.as_ast/2](#earmarkparseras_ast2)
  - [EarmarkParser.version/0](#earmarkparserversion0)
- [Contributing](#contributing)
- [Author](#author)
- [LICENSE](#license)

## Usage

### EarmarkParser


### API

#### EarmarkParser.as_ast

This is the structure of the result of `as_ast`.

    {:ok, ast, []}                   = EarmarkParser.as_ast(markdown)
    {:ok, ast, deprecation_messages} = EarmarkParser.as_ast(markdown)
    {:error, ast, error_messages}    = EarmarkParser.as_ast(markdown)

For examples see the functiondoc below.

#### Options

Options can be passed into `as_ast/2` according to the documentation of `EarmarkParser.Options`.

    {status, ast, errors} = EarmarkParser.as_ast(markdown, options)

## Supports

Standard [Gruber markdown][gruber].

[gruber]: <http://daringfireball.net/projects/markdown/syntax>

## Extensions

### Links

#### Links supported by default

##### Oneline HTML Link tags

```elixir
    iex(1)> EarmarkParser.as_ast(~s{<a href="href">link</a>})
    {:ok, [{"a", [{"href", "href"}], ["link"], %{verbatim: true}}], []}
```

##### Markdown links

New style ...

```elixir
    iex(2)> EarmarkParser.as_ast(~s{[title](destination)})
    {:ok,  [{"p", [], [{"a", [{"href", "destination"}], ["title"], %{}}], %{}}], []}
```

and old style

```elixir
    iex(3)> EarmarkParser.as_ast("[foo]: /url \"title\"\n\n[foo]\n")
    {:ok, [{"p", [], [{"a", [{"href", "/url"}, {"title", "title"}], ["foo"], %{}}], %{}}], []}
```

#### Autolinks

```elixir
    iex(4)> EarmarkParser.as_ast("<https://elixir-lang.com>")
    {:ok, [{"p", [], [{"a", [{"href", "https://elixir-lang.com"}], ["https://elixir-lang.com"], %{}}], %{}}], []}
```

#### Additional link parsing via options


#### Pure links

**N.B.** that the `pure_links` option is `true` by default

```elixir
    iex(5)> EarmarkParser.as_ast("https://github.com")
    {:ok, [{"p", [], [{"a", [{"href", "https://github.com"}], ["https://github.com"], %{}}], %{}}], []}
```

But can be deactivated

```elixir
    iex(6)> EarmarkParser.as_ast("https://github.com", pure_links: false)
    {:ok, [{"p", [], ["https://github.com"], %{}}], []}
```


  #### Wikilinks...

  are disabled by default

```elixir
    iex(7)> EarmarkParser.as_ast("[[page]]")
    {:ok, [{"p", [], ["[[page]]"], %{}}], []}
```

  and can be enabled

```elixir
    iex(8)> EarmarkParser.as_ast("[[page]]", wikilinks: true)
    {:ok, [{"p", [], [{"a", [{"href", "page"}], ["page"], %{wikilink: true}}], %{}}], []}
```



### Github Flavored Markdown

GFM is supported by default, however as GFM is a moving target and all GFM extension do not make sense in a general context, EarmarkParser does not support all of it, here is a list of what is supported:

#### Strike Through

```elixir
    iex(9)> EarmarkParser.as_ast("~~hello~~")
    {:ok, [{"p", [], [{"del", [], ["hello"], %{}}], %{}}], []}
```

#### Syntax Highlighting

All backquoted or fenced code blocks with a language string are rendered with the given
language as a _class_ attribute of the _code_ tag.

For example:

```elixir
    iex(10)> [
    ...(10)>    "```elixir",
    ...(10)>    " @tag :hello",
    ...(10)>    "```"
    ...(10)> ] |> EarmarkParser.as_ast()
    {:ok, [{"pre", [], [{"code", [{"class", "elixir"}], [" @tag :hello"], %{}}], %{}}], []}
```

will be rendered as shown in the doctest above.

If you want to integrate with a syntax highlighter with different conventions you can add more classes by specifying prefixes that will be
put before the language string.

Prism.js for example needs a class `language-elixir`. In order to achieve that goal you can add `language-`
as a `code_class_prefix` to `EarmarkParser.Options`.

In the following example we want more than one additional class, so we add more prefixes.

```elixir
    iex(11)> [
    ...(11)>    "```elixir",
    ...(11)>    " @tag :hello",
    ...(11)>    "```"
    ...(11)> ] |> EarmarkParser.as_ast(%EarmarkParser.Options{code_class_prefix: "lang- language-"})
    {:ok, [{"pre", [], [{"code", [{"class", "elixir lang-elixir language-elixir"}], [" @tag :hello"], %{}}], %{}}], []}
```


#### Tables

Are supported as long as they are preceded by an empty line.

    State | Abbrev | Capital
    ----: | :----: | -------
    Texas | TX     | Austin
    Maine | ME     | Augusta

Tables may have leading and trailing vertical bars on each line

    | State | Abbrev | Capital |
    | ----: | :----: | ------- |
    | Texas | TX     | Austin  |
    | Maine | ME     | Augusta |

Tables need not have headers, in which case all column alignments
default to left.

    | Texas | TX     | Austin  |
    | Maine | ME     | Augusta |

Currently we assume there are always spaces around interior vertical unless
there are exterior bars.

However in order to be more GFM compatible the `gfm_tables: true` option
can be used to interpret only interior vertical bars as a table if a separation
line is given, therefore

     Language|Rating
     --------|------
     Elixir  | awesome

is a table (if and only if `gfm_tables: true`) while

     Language|Rating
     Elixir  | awesome

never is.

#### HTML Blocks

HTML is not parsed recursively or detected in all conditions right now, though GFM compliance
is a goal.

But for now the following holds:

A HTML Block defined by a tag starting a line and the same tag starting a different line is parsed
as one HTML AST node, marked with %{verbatim: true}

E.g.

```elixir
    iex(12)> lines = [ "<div><span>", "some</span><text>", "</div>more text" ]
    ...(12)> EarmarkParser.as_ast(lines)
    {:ok, [{"div", [], ["<span>", "some</span><text>"], %{verbatim: true}}, "more text"], []}
```

And a line starting with an opening tag and ending with the corresponding closing tag is parsed in similar
fashion

```elixir
    iex(13)> EarmarkParser.as_ast(["<span class=\"superspan\">spaniel</span>"])
    {:ok, [{"span", [{"class", "superspan"}], ["spaniel"], %{verbatim: true}}], []}
```

What is HTML?

We differ from strict GFM by allowing **all** tags not only HTML5 tags this holds for one liners....

```elixir
    iex(14)> {:ok, ast, []} = EarmarkParser.as_ast(["<stupid />", "<not>better</not>"])
    ...(14)> ast
    [
      {"stupid", [], [], %{verbatim: true}},
      {"not", [], ["better"], %{verbatim: true}}]
```

and for multi line blocks

```elixir
    iex(15)> {:ok, ast, []} = EarmarkParser.as_ast([ "<hello>", "world", "</hello>"])
    ...(15)> ast
    [{"hello", [], ["world"], %{verbatim: true}}]
```

#### HTML Comments

Are recognized if they start a line (after ws and are parsed until the next `-->` is found
all text after the next '-->' is ignored

E.g.

```elixir
    iex(16)> EarmarkParser.as_ast(" <!-- Comment\ncomment line\ncomment --> text -->\nafter")
    {:ok, [{:comment, [], [" Comment", "comment line", "comment "], %{comment: true}}, {"p", [], ["after"], %{}}], []}
```



### Adding Attributes with the IAL extension

#### To block elements

HTML attributes can be added to any block-level element. We use
the Kramdown syntax: add the line `{:` _attrs_ `}` following the block.

```elixir
    iex(17)> markdown = ["# Headline", "{:.from-next-line}"]
    ...(17)> as_ast(markdown)
    {:ok, [{"h1", [{"class", "from-next-line"}], ["Headline"], %{}}], []}
```

Headers can also have the IAL string at the end of the line

```elixir
    iex(18)> markdown = ["# Headline{:.from-same-line}"]
    ...(18)> as_ast(markdown)
    {:ok, [{"h1", [{"class", "from-same-line"}], ["Headline"], %{}}], []}
```

A special use case is headers inside blockquotes which allow for some nifty styling in `ex_doc`*
see [this PR](https://github.com/elixir-lang/ex_doc/pull/1400) if you are interested in the technical
details

```elixir
    iex(19)> markdown = ["> # Headline{:.warning}"]
    ...(19)> as_ast(markdown)
    {:ok, [{"blockquote", [], [{"h1", [{"class", "warning"}], ["Headline"], %{}}], %{}}], []}
```

This also works for headers inside lists

```elixir
    iex(20)> markdown = ["- # Headline{:.warning}"]
    ...(20)> as_ast(markdown)
    {:ok, [{"ul", [], [{"li", [], [{"h1", [{"class", "warning"}], ["Headline"], %{}}], %{}}], %{}}], []}
```

It still works for inline code, as it did before

```elixir
    iex(21)> markdown = "`Enum.map`{:lang=elixir}"
    ...(21)> as_ast(markdown)
    {:ok, [{"p", [], [{"code", [{"class", "inline"}, {"lang", "elixir"}], ["Enum.map"], %{}}], %{}}], []}
```


_attrs_ can be one or more of:

  * `.className`
  * `#id`
  * name=value, name="value", or name='value'

For example:

    # Warning
    {: .red}

    Do not turn off the engine
    if you are at altitude.
    {: .boxed #warning spellcheck="true"}

#### To links or images

It is possible to add IAL attributes to generated links or images in the following
format.

```elixir
    iex(22)> markdown = "[link](url) {: .classy}"
    ...(22)> EarmarkParser.as_ast(markdown)
    { :ok, [{"p", [], [{"a", [{"class", "classy"}, {"href", "url"}], ["link"], %{}}], %{}}], []}
```

For both cases, malformed attributes are ignored and warnings are issued.

```elixir
    iex(23)> [ "Some text", "{:hello}" ] |> Enum.join("\n") |> EarmarkParser.as_ast()
    {:error, [{"p", [], ["Some text"], %{}}], [{:warning, 2,"Illegal attributes [\"hello\"] ignored in IAL"}]}
```

It is possible to escape the IAL in both forms if necessary

```elixir
    iex(24)> markdown = "[link](url)\\{: .classy}"
    ...(24)> EarmarkParser.as_ast(markdown)
    {:ok, [{"p", [], [{"a", [{"href", "url"}], ["link"], %{}}, "{: .classy}"], %{}}], []}
```

This of course is not necessary in code blocks or text lines
containing an IAL-like string, as in the following example

```elixir
    iex(25)> markdown = "hello {:world}"
    ...(25)> EarmarkParser.as_ast(markdown)
    {:ok, [{"p", [], ["hello {:world}"], %{}}], []}
```

## Limitations

  * Block-level HTML is correctly handled only if each HTML
    tag appears on its own line. So

        <div>
        <div>
        hello
        </div>
        </div>

    will work. However. the following won't

        <div>
        hello</div>

  * John Gruber's tests contain an ambiguity when it comes to
    lines that might be the start of a list inside paragraphs.

    One test says that

        This is the text
        * of a paragraph
        that I wrote

    is a single paragraph. The "*" is not significant. However, another
    test has

        *   A list item
            * an another

    and expects this to be a nested list. But, in reality, the second could just
    be the continuation of a paragraph.

    I've chosen always to use the second interpretation—a line that looks like
    a list item will always be a list item.

  * Rendering of block and inline elements.

    Block or void HTML elements that are at the absolute beginning of a line end
    the preceding paragraph.

    Thusly

        mypara
        <hr />

    Becomes

        <p>mypara</p>
        <hr />

    While

        mypara
         <hr />

    will be transformed into

        <p>mypara
         <hr /></p>

## Timeouts

By default, that is if the `timeout` option is not set EarmarkParser uses parallel mapping as implemented in `EarmarkParser.pmap/2`,
which uses `Task.await` with its default timeout of 5000ms.

In rare cases that might not be enough.

By indicating a longer `timeout` option in milliseconds EarmarkParser will use parallel mapping as implemented in `EarmarkParser.pmap/3`,
which will pass `timeout` to `Task.await`.

In both cases one can override the mapper function with either the `mapper` option (used if and only if `timeout` is nil) or the
`mapper_with_timeout` function (used otherwise).

## Annotations

**N.B.** this is an experimental feature from v1.4.16-pre on and might change or be removed again

The idea is that each markdown line can be annotated, as such annotations change the semantics of Markdown
they have to be enabled with the `annotations` option.

If the `annotations` option is set to a string (only one string is supported right now, but a list might
be implemented later on, hence the name), the last occurance of that string in a line and all text following
it will be added to the line as an annotation.

Depending on how that line will eventually be parsed, this annotation will be added to the meta map (the 4th element
in an AST quadruple) with the key `:annotation`

In the current version the annotation will only be applied to verbatim HTML tags and paragraphs

Let us show some examples now:

### Annotated Paragraphs

```elixir
    iex(26)> as_ast("hello %> annotated", annotations: "%>")
    {:ok, [{"p", [], ["hello "], %{annotation: "%> annotated"}}], []}
```

If we annotate more than one line in a para the first annotation takes precedence

```elixir
    iex(27)> as_ast("hello %> annotated\nworld %> discarded", annotations: "%>")
    {:ok, [{"p", [], ["hello \nworld "], %{annotation: "%> annotated"}}], []}
```

### Annotated HTML elements

In one line

```elixir
    iex(28)> as_ast("<span>One Line</span> // a span", annotations: "//")
    {:ok, [{"span", [], ["One Line"], %{annotation: "// a span", verbatim: true}}], []}
```

or block elements

```elixir
    iex(29)> [
    ...(29)> "<div> : annotation",
    ...(29)> "  <span>text</span>",
    ...(29)> "</div> : discarded"
    ...(29)> ] |> as_ast(annotations: " : ")
    {:ok, [{"div", [], ["  <span>text</span>"], %{annotation: " : annotation", verbatim: true}}], []}
```

### Commenting your Markdown

Although many markdown elements do not support annotations yet, they can be used to comment your markdown, w/o cluttering
the generated AST with comments

```elixir
    iex(30)> [
    ...(30)> "# Headline --> first line",
    ...(30)> "- item1 --> a list item",
    ...(30)> "- item2 --> another list item",
    ...(30)> "",
    ...(30)> "<http://somewhere/to/go> --> do not go there"
    ...(30)> ] |> as_ast(annotations: "-->")
    {:ok, [
      {"h1", [], ["Headline"], %{}},
      {"ul", [], [{"li", [], ["item1 "], %{}}, {"li", [], ["item2 "], %{}}], %{}},
      {"p", [], [{"a", [{"href", "http://somewhere/to/go"}], ["http://somewhere/to/go"], %{}}, " "], %{annotation: "--> do not go there"}}
      ], []
     }
```


### EarmarkParser.as_ast/2

    iex(31)> markdown = "My `code` is **best**"
    ...(31)> {:ok, ast, []} = EarmarkParser.as_ast(markdown)
    ...(31)> ast
    [{"p", [], ["My ", {"code", [{"class", "inline"}], ["code"], %{}}, " is ", {"strong", [], ["best"], %{}}], %{}}]



```elixir
    iex(32)> markdown = "```elixir\nIO.puts 42\n```"
    ...(32)> {:ok, ast, []} = EarmarkParser.as_ast(markdown, code_class_prefix: "lang-")
    ...(32)> ast
    [{"pre", [], [{"code", [{"class", "elixir lang-elixir"}], ["IO.puts 42"], %{}}], %{}}]
```

**Rationale**:

The AST is exposed in the spirit of [Floki's](https://hex.pm/packages/floki).

### EarmarkParser.version/0

  Accesses current hex version of the `EarmarkParser` application. Convenience for
  `iex` usage.



## Contributing

Pull Requests are happily accepted.

Please be aware of one _caveat_ when correcting/improving `README.md`.

The `README.md` is generated by the mix task `readme` from `README.template` and
docstrings by means of `%moduledoc` or `%functiondoc` directives.

Please identify the origin of the generated text you want to correct and then
apply your changes there.

Then issue the mix task `readme`, this is important to have a correctly updated `README.md` after the merge of
your PR.

Thank you all who have already helped with Earmark/EarmarkParser, your names are duely noted in [RELEASE.md](RELEASE.md).

## Author

Copyright © 2014,5,6,7,8,9;2020 Dave Thomas, The Pragmatic Programmers
@/+pragdave,  dave@pragprog.com
Copyright © 2020 Robert Dober
robert.dober@gmail.com

## LICENSE

Same as Elixir, which is Apache License v2.0. Please refer to [LICENSE](LICENSE) for details.

<!-- SPDX-License-Identifier: Apache-2.0 -->
