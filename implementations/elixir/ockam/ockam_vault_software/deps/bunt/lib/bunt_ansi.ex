defmodule Bunt.ANSI.Sequence do
  @moduledoc false

  defmacro defalias(alias_name, original_name) do
    quote bind_quoted: [alias_name: alias_name, original_name: original_name] do
      def unquote(alias_name)() do
        unquote(original_name)()
      end

      defp format_sequence(unquote(alias_name)) do
        unquote(original_name)()
      end
    end
  end

  defmacro defsequence(name, code, prefix \\ "", terminator \\ "m") do
    quote bind_quoted: [name: name, code: code, prefix: prefix, terminator: terminator] do
      def unquote(name)() do
        "\e[#{unquote(prefix)}#{unquote(code)}#{unquote(terminator)}"
      end

      defp format_sequence(unquote(name)) do
        unquote(name)()
      end
    end
  end
end

defmodule Bunt.ANSI do
  @moduledoc """
  Functionality to render ANSI escape sequences.

  [ANSI escape sequences](https://en.wikipedia.org/wiki/ANSI_escape_code)
  are characters embedded in text used to control formatting, color, and
  other output options on video text terminals.
  """

  import Bunt.ANSI.Sequence

  @color_tuples [
    {nil,       :color16, 16, {0, 0, 0}},
    {nil,       :color17, 17, {0, 0, 95}},
    {"darkblue",       :color18, 18, {0, 0, 135}},
    {nil,       :color19, 19, {0, 0, 175}},
    {"mediumblue",       :color20, 20, {0, 0, 215}},
    {nil,       :color21, 21, {0, 0, 255}},
    {"darkgreen",     :color22, 22, {0, 95, 0}},
    {"darkslategray", :color23, 23, {0, 95, 95}},
    {nil,       :color24, 24, {0, 95, 135}},
    {nil,       :color25, 25, {0, 95, 175}},
    {nil,       :color26, 26, {0, 95, 215}},
    {nil,       :color27, 27, {0, 95, 255}},
    {nil,       :color28, 28, {0, 135, 0}},
    {nil,       :color29, 29, {0, 135, 95}},
    {"darkcyan",       :color30, 30, {0, 135, 135}},
    {nil,       :color31, 31, {0, 135, 175}},
    {nil,       :color32, 32, {0, 135, 215}},
    {nil,       :color33, 33, {0, 135, 255}},
    {nil,       :color34, 34, {0, 175, 0}},
    {nil,       :color35, 35, {0, 175, 95}},
    {nil,       :color36, 36, {0, 175, 135}},
    {nil,       :color37, 37, {0, 175, 175}},
    {nil,       :color38, 38, {0, 175, 215}},
    {"deepskyblue",       :color39, 39, {0, 175, 255}},
    {nil,       :color40, 40, {0, 215, 0}},
    {nil,       :color41, 41, {0, 215, 95}},
    {nil,       :color42, 42, {0, 215, 135}},
    {nil,       :color43, 43, {0, 215, 175}},
    {nil,       :color44, 44, {0, 215, 215}},
    {nil,       :color45, 45, {0, 215, 255}},
    {nil,       :color46, 46, {0, 255, 0}},
    {nil,       :color47, 47, {0, 255, 95}},
    {"springgreen",       :color48, 48, {0, 255, 135}},
    {nil,       :color49, 49, {0, 255, 175}},
    {nil,       :color50, 50, {0, 255, 215}},
    {"aqua",       :color51, 51, {0, 255, 255}},
    {nil,       :color52, 52, {95, 0, 0}},
    {nil,       :color53, 53, {95, 0, 95}},
    {nil,       :color54, 54, {95, 0, 135}},
    {nil,       :color55, 55, {95, 0, 175}},
    {nil,       :color56, 56, {95, 0, 215}},
    {nil,       :color57, 57, {95, 0, 255}},
    {nil,       :color58, 58, {95, 95, 0}},
    {"dimgray",       :color59, 59, {95, 95, 95}},
    {nil,       :color60, 60, {95, 95, 135}},
    {nil,       :color61, 61, {95, 95, 175}},
    {nil,       :color62, 62, {95, 95, 215}},
    {nil,       :color63, 63, {95, 95, 255}},
    {nil,       :color64, 64, {95, 135, 0}},
    {nil,       :color65, 65, {95, 135, 95}},
    {nil,       :color66, 66, {95, 135, 135}},
    {"steelblue",       :color67, 67, {95, 135, 175}},
    {nil,       :color68, 68, {95, 135, 215}},
    {nil,       :color69, 69, {95, 135, 255}},
    {nil,       :color70, 70, {95, 175, 0}},
    {nil,       :color71, 71, {95, 175, 95}},
    {nil,       :color72, 72, {95, 175, 135}},
    {nil,       :color73, 73, {95, 175, 175}},
    {nil,       :color74, 74, {95, 175, 215}},
    {nil,       :color75, 75, {95, 175, 255}},
    {nil,       :color76, 76, {95, 215, 0}},
    {nil,       :color77, 77, {95, 215, 95}},
    {nil,       :color78, 78, {95, 215, 135}},
    {nil,       :color79, 79, {95, 215, 175}},
    {nil,       :color80, 80, {95, 215, 215}},
    {nil,       :color81, 81, {95, 215, 255}},
    {nil,       :color82, 82, {95, 255, 0}},
    {nil,       :color83, 83, {95, 255, 95}},
    {nil,       :color84, 84, {95, 255, 135}},
    {nil,       :color85, 85, {95, 255, 175}},
    {nil,       :color86, 86, {95, 255, 215}},
    {nil,       :color87, 87, {95, 255, 255}},
    {"darkred", :color88, 88, {135, 0, 0}},
    {nil,       :color89, 89, {135, 0, 95}},
    {"darkmagenta",       :color90, 90, {135, 0, 135}},
    {nil,       :color91, 91, {135, 0, 175}},
    {nil,       :color92, 92, {135, 0, 215}},
    {nil,       :color93, 93, {135, 0, 255}},
    {nil,       :color94, 94, {135, 95, 0}},
    {nil,       :color95, 95, {135, 95, 95}},
    {nil,       :color96, 96, {135, 95, 135}},
    {nil,       :color97, 97, {135, 95, 175}},
    {nil,       :color98, 98, {135, 95, 215}},
    {nil,       :color99, 99, {135, 95, 255}},
    {"olive",   :color100, 100, {135, 135, 0}},
    {nil,       :color101, 101, {135, 135, 95}},
    {nil,       :color102, 102, {135, 135, 135}},
    {nil,       :color103, 103, {135, 135, 175}},
    {nil,       :color104, 104, {135, 135, 215}},
    {nil,       :color105, 105, {135, 135, 255}},
    {nil,       :color106, 106, {135, 175, 0}},
    {nil,       :color107, 107, {135, 175, 95}},
    {nil,       :color108, 108, {135, 175, 135}},
    {nil,       :color109, 109, {135, 175, 175}},
    {nil,       :color110, 110, {135, 175, 215}},
    {nil,       :color111, 111, {135, 175, 255}},
    {nil,       :color112, 112, {135, 215, 0}},
    {nil,       :color113, 113, {135, 215, 95}},
    {nil,       :color114, 114, {135, 215, 135}},
    {nil,       :color115, 115, {135, 215, 175}},
    {nil,       :color116, 116, {135, 215, 215}},
    {nil,       :color117, 117, {135, 215, 255}},
    {"chartreuse",       :color118, 118, {135, 255, 0}},
    {nil,       :color119, 119, {135, 255, 95}},
    {nil,       :color120, 120, {135, 255, 135}},
    {nil,       :color121, 121, {135, 255, 175}},
    {"aquamarine",       :color122, 122, {135, 255, 215}},
    {nil,       :color123, 123, {135, 255, 255}},
    {nil,       :color124, 124, {175, 0, 0}},
    {nil,       :color125, 125, {175, 0, 95}},
    {nil,       :color126, 126, {175, 0, 135}},
    {nil,       :color127, 127, {175, 0, 175}},
    {nil,       :color128, 128, {175, 0, 215}},
    {nil,       :color129, 129, {175, 0, 255}},
    {nil,       :color130, 130, {175, 95, 0}},
    {nil,       :color131, 131, {175, 95, 95}},
    {nil,       :color132, 132, {175, 95, 135}},
    {nil,       :color133, 133, {175, 95, 175}},
    {nil,       :color134, 134, {175, 95, 215}},
    {nil,       :color135, 135, {175, 95, 255}},
    {nil,       :color136, 136, {175, 135, 0}},
    {nil,       :color137, 137, {175, 135, 95}},
    {nil,       :color138, 138, {175, 135, 135}},
    {nil,       :color139, 139, {175, 135, 175}},
    {nil,       :color140, 140, {175, 135, 215}},
    {nil,       :color141, 141, {175, 135, 255}},
    {nil,       :color142, 142, {175, 175, 0}},
    {nil,       :color143, 143, {175, 175, 95}},
    {nil,       :color144, 144, {175, 175, 135}},
    {nil,       :color145, 145, {175, 175, 175}},
    {nil,       :color146, 146, {175, 175, 215}},
    {nil,       :color147, 147, {175, 175, 255}},
    {nil,       :color148, 148, {175, 215, 0}},
    {nil,       :color149, 149, {175, 215, 95}},
    {nil,       :color150, 150, {175, 215, 135}},
    {nil,       :color151, 151, {175, 215, 175}},
    {nil,       :color152, 152, {175, 215, 215}},
    {nil,       :color153, 153, {175, 215, 255}},
    {"greenyellow",       :color154, 154, {175, 255, 0}},
    {nil,       :color155, 155, {175, 255, 95}},
    {nil,       :color156, 156, {175, 255, 135}},
    {nil,       :color157, 157, {175, 255, 175}},
    {nil,       :color158, 158, {175, 255, 215}},
    {nil,       :color159, 159, {175, 255, 255}},
    {nil,       :color160, 160, {215, 0, 0}},
    {nil,       :color161, 161, {215, 0, 95}},
    {nil,       :color162, 162, {215, 0, 135}},
    {nil,       :color163, 163, {215, 0, 175}},
    {nil,       :color164, 164, {215, 0, 215}},
    {nil,       :color165, 165, {215, 0, 255}},
    {nil,       :color166, 166, {215, 95, 0}},
    {nil,       :color167, 167, {215, 95, 95}},
    {nil,       :color168, 168, {215, 95, 135}},
    {nil,       :color169, 169, {215, 95, 175}},
    {nil,       :color170, 170, {215, 95, 215}},
    {nil,       :color171, 171, {215, 95, 255}},
    {"chocolate",       :color172, 172, {215, 135, 0}},
    {nil,       :color173, 173, {215, 135, 95}},
    {nil,       :color174, 174, {215, 135, 135}},
    {nil,       :color175, 175, {215, 135, 175}},
    {nil,       :color176, 176, {215, 135, 215}},
    {nil,       :color177, 177, {215, 135, 255}},
    {"goldenrod",       :color178, 178, {215, 175, 0}},
    {nil,       :color179, 179, {215, 175, 95}},
    {nil,       :color180, 180, {215, 175, 135}},
    {nil,       :color181, 181, {215, 175, 175}},
    {nil,       :color182, 182, {215, 175, 215}},
    {nil,       :color183, 183, {215, 175, 255}},
    {nil,       :color184, 184, {215, 215, 0}},
    {nil,       :color185, 185, {215, 215, 95}},
    {nil,       :color186, 186, {215, 215, 135}},
    {nil,       :color187, 187, {215, 215, 175}},
    {"lightgray", :color188, 188, {215, 215, 215}},
    {nil,       :color189, 189, {215, 215, 255}},
    {nil,       :color190, 190, {215, 255, 0}},
    {nil,       :color191, 191, {215, 255, 95}},
    {nil,       :color192, 192, {215, 255, 135}},
    {nil,       :color193, 193, {215, 255, 175}},
    {"beige",       :color194, 194, {215, 255, 215}},
    {"lightcyan",       :color195, 195, {215, 255, 255}},
    {nil,       :color196, 196, {255, 0, 0}},
    {nil,       :color197, 197, {255, 0, 95}},
    {nil,       :color198, 198, {255, 0, 135}},
    {nil,       :color199, 199, {255, 0, 175}},
    {nil,       :color200, 200, {255, 0, 215}},
    {"fuchsia",       :color201, 201, {255, 0, 255}},
    {"orangered", :color202, 202, {255, 95, 0}},
    {nil,       :color203, 203, {255, 95, 95}},
    {nil,       :color204, 204, {255, 95, 135}},
    {"hotpink", :color205, 205, {255, 95, 175}},
    {nil,       :color206, 206, {255, 95, 215}},
    {nil,       :color207, 207, {255, 95, 255}},
    {"darkorange", :color208, 208, {255, 135, 0}},
    {"coral",       :color209, 209, {255, 135, 95}},
    {nil,       :color210, 210, {255, 135, 135}},
    {nil,       :color211, 211, {255, 135, 175}},
    {nil,       :color212, 212, {255, 135, 215}},
    {nil,       :color213, 213, {255, 135, 255}},
    {"orange",  :color214, 214, {255, 175, 0}},
    {nil,       :color215, 215, {255, 175, 95}},
    {nil,       :color216, 216, {255, 175, 135}},
    {nil,       :color217, 217, {255, 175, 175}},
    {nil,       :color218, 218, {255, 175, 215}},
    {nil,       :color219, 219, {255, 175, 255}},
    {"gold",    :color220, 220, {255, 215, 0}},
    {nil,       :color221, 221, {255, 215, 95}},
    {"khaki",   :color222, 222, {255, 215, 135}},
    {"moccasin",       :color223, 223, {255, 215, 175}},
    {"mistyrose", :color224, 224, {255, 215, 215}},
    {nil,       :color225, 225, {255, 215, 255}},
    {nil,       :color226, 226, {255, 255, 0}},
    {nil,       :color227, 227, {255, 255, 95}},
    {nil,       :color228, 228, {255, 255, 135}},
    {nil,       :color229, 229, {255, 255, 175}},
    {"lightyellow", :color230, 230, {255, 255, 215}},
    {nil,       :color231, 231, {255, 255, 255}},
    {nil,       :color232, 232, {255, 255, 255}},
    {nil,       :color233, 233, {255, 255, 255}},
    {nil,       :color234, 234, {255, 255, 255}},
    {nil,       :color235, 235, {255, 255, 255}},
    {nil,       :color236, 236, {255, 255, 255}},
    {nil,       :color237, 237, {255, 255, 255}},
    {nil,       :color238, 238, {255, 255, 255}},
    {nil,       :color239, 239, {255, 255, 255}},
    {nil,       :color240, 240, {255, 255, 255}},
    {nil,       :color241, 241, {255, 255, 255}},
    {nil,       :color242, 242, {255, 255, 255}},
    {nil,       :color243, 243, {255, 255, 255}},
    {nil,       :color244, 244, {255, 255, 255}},
    {nil,       :color245, 245, {255, 255, 255}},
    {nil,       :color246, 246, {255, 255, 255}},
    {nil,       :color247, 247, {255, 255, 255}},
    {nil,       :color248, 248, {255, 255, 255}},
    {nil,       :color249, 249, {255, 255, 255}},
    {nil,       :color250, 250, {255, 255, 255}},
    {nil,       :color251, 251, {255, 255, 255}},
    {nil,       :color252, 252, {255, 255, 255}},
    {nil,       :color253, 253, {255, 255, 255}},
    {nil,       :color254, 254, {255, 255, 255}},
    {nil,       :color255, 255, {255, 255, 255}},
  ]

  def color_tuples, do: @color_tuples

  for {name, color, code, _} <- @color_tuples do
    @doc "Sets foreground color to #{color}"
    defsequence color, code, "38;5;"

    @doc "Sets background color to #{color}"
    defsequence :"#{color}_background", code, "48;5;"
    if name do
      @doc "Sets foreground color to #{name}"
      defsequence :"#{name}", code, "38;5;"

      @doc "Sets background color to #{name}"
      defsequence :"#{name}_background", code, "48;5;"
    end
  end

  @color_aliases Application.get_env(:bunt, :color_aliases, [])
  def color_aliases, do: @color_aliases

  for {alias_name, original_name} <- @color_aliases do
    defalias alias_name, original_name
    defalias :"#{alias_name}_background", :"#{original_name}_background"
  end




  @typep ansicode :: atom()
  @typep ansilist :: maybe_improper_list(char() | ansicode() | binary() | ansilist(), binary() | ansicode() | [])
  @type  ansidata :: ansilist() | ansicode() | binary()

  @doc """
  Checks if ANSI coloring is supported and enabled on this machine.

  This function simply reads the configuration value for
  `:ansi_enabled` in the `:elixir` application. The value is by
  default `false` unless Elixir can detect during startup that
  both `stdout` and `stderr` are terminals.
  """
  @spec enabled? :: boolean
  def enabled? do
    Application.get_env(:elixir, :ansi_enabled, false)
  end

  @doc "Resets all attributes"
  defsequence :reset, 0

  @doc "Bright (increased intensity) or Bold"
  defsequence :bright, 1

  @doc "Faint (decreased intensity), not widely supported"
  defsequence :faint, 2

  @doc "Italic: on. Not widely supported. Sometimes treated as inverse"
  defsequence :italic, 3

  @doc "Underline: Single"
  defsequence :underline, 4

  @doc "Blink: Slow. Less than 150 per minute"
  defsequence :blink_slow, 5

  @doc "Blink: Rapid. MS-DOS ANSI.SYS; 150 per minute or more; not widely supported"
  defsequence :blink_rapid, 6

  @doc "Image: Negative. Swap foreground and background"
  defsequence :inverse, 7

  @doc "Image: Negative. Swap foreground and background"
  defsequence :reverse, 7

  @doc "Conceal. Not widely supported"
  defsequence :conceal, 8

  @doc "Crossed-out. Characters legible, but marked for deletion. Not widely supported"
  defsequence :crossed_out, 9

  @doc "Sets primary (default) font"
  defsequence :primary_font, 10

  for font_n <- [1, 2, 3, 4, 5, 6, 7, 8, 9] do
    @doc "Sets alternative font #{font_n}"
    defsequence :"font_#{font_n}", font_n + 10
  end

  @doc "Normal color or intensity"
  defsequence :normal, 22

  @doc "Not italic"
  defsequence :not_italic, 23

  @doc "Underline: None"
  defsequence :no_underline, 24

  @doc "Blink: off"
  defsequence :blink_off, 25

  colors = [:black, :red, :green, :yellow, :blue, :magenta, :cyan, :white]

  for {color, code} <- Enum.with_index(colors) do
    @doc "Sets foreground color to #{color}"
    defsequence color, code + 30

    @doc "Sets background color to #{color}"
    defsequence :"#{color}_background", code + 40
  end

  @doc "Default text color"
  defsequence :default_color, 39

  @doc "Default background color"
  defsequence :default_background, 49

  @doc "Framed"
  defsequence :framed, 51

  @doc "Encircled"
  defsequence :encircled, 52

  @doc "Overlined"
  defsequence :overlined, 53

  @doc "Not framed or encircled"
  defsequence :not_framed_encircled, 54

  @doc "Not overlined"
  defsequence :not_overlined, 55

  @doc "Sends cursor home"
  defsequence :home, "", "H"

  @doc "Clears screen"
  defsequence :clear, "2", "J"

  @doc "Clears line"
  defsequence :clear_line, "2", "K"

  defp format_sequence(other) do
    raise ArgumentError, "invalid ANSI sequence specification: #{other}"
  end

  @doc ~S"""
  Formats a chardata-like argument by converting named ANSI sequences into actual
  ANSI codes.

  The named sequences are represented by atoms.

  It will also append an `IO.ANSI.reset/0` to the chardata when a conversion is
  performed. If you don't want this behaviour, use `format_fragment/2`.

  An optional boolean parameter can be passed to enable or disable
  emitting actual ANSI codes. When `false`, no ANSI codes will emitted.
  By default checks if ANSI is enabled using the `enabled?/0` function.

  ## Examples

      iex> IO.ANSI.format(["Hello, ", :red, :bright, "world!"], true)
      [[[[[[], "Hello, "] | "\e[31m"] | "\e[1m"], "world!"] | "\e[0m"]

  """
  def format(chardata, emit \\ enabled?()) when is_boolean(emit) do
    do_format(chardata, [], [], emit, :maybe)
  end

  @doc ~S"""
  Formats a chardata-like argument by converting named ANSI sequences into actual
  ANSI codes.

  The named sequences are represented by atoms.

  An optional boolean parameter can be passed to enable or disable
  emitting actual ANSI codes. When `false`, no ANSI codes will emitted.
  By default checks if ANSI is enabled using the `enabled?/0` function.

  ## Examples

      iex> IO.ANSI.format_fragment([:bright, 'Word'], true)
      [[[[[[] | "\e[1m"], 87], 111], 114], 100]

  """
  def format_fragment(chardata, emit \\ enabled?()) when is_boolean(emit) do
    do_format(chardata, [], [], emit, false)
  end

  defp do_format([term | rest], rem, acc, emit, append_reset) do
    do_format(term, [rest | rem], acc, emit, append_reset)
  end

  defp do_format(term, rem, acc, true, append_reset) when is_atom(term) do
    do_format([], rem, [acc | format_sequence(term)], true, !!append_reset)
  end

  defp do_format(term, rem, acc, false, append_reset) when is_atom(term) do
    do_format([], rem, acc, false, append_reset)
  end

  defp do_format(term, rem, acc, emit, append_reset) when not is_list(term) do
    do_format([], rem, [acc | [term]], emit, append_reset)
  end

  defp do_format([], [next | rest], acc, emit, append_reset) do
    do_format(next, rest, acc, emit, append_reset)
  end

  defp do_format([], [], acc, true, true) do
    [acc | IO.ANSI.reset]
  end

  defp do_format([], [], acc, _emit, _append_reset) do
    acc
  end
end
