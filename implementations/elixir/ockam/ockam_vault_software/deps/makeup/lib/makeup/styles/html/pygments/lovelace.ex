
defmodule Makeup.Styles.HTML.LovelaceStyle do
  @moduledoc false

  @styles %{
    :error => "bg:#a848a8",
    :keyword => "#2838b0",
    :keyword_constant => "italic #444444",
    :keyword_declaration => "italic",
    :keyword_type => "italic",
    :name_attribute => "#388038",
    :name_builtin => "#388038",
    :name_builtin_pseudo => "italic",
    :name_class => "#287088",
    :name_constant => "#b85820",
    :name_decorator => "#287088",
    :name_entity => "#709030",
    :name_exception => "#908828",
    :name_function => "#785840",
    :name_function_magic => "#b85820",
    :name_label => "#289870",
    :name_namespace => "#289870",
    :name_tag => "#2838b0",
    :name_variable => "#b04040",
    :name_variable_global => "#908828",
    :name_variable_magic => "#b85820",
    :string => "#b83838",
    :string_affix => "#444444",
    :string_char => "#a848a8",
    :string_delimiter => "#b85820",
    :string_doc => "italic #b85820",
    :string_escape => "#709030",
    :string_interpol => "underline",
    :string_other => "#a848a8",
    :string_regex => "#a848a8",
    :number => "#444444",
    :operator => "#666666",
    :operator_word => "#a848a8",
    :punctuation => "#888888",
    :comment => "italic #888888",
    :comment_hashbang => "#287088",
    :comment_multiline => "#888888",
    :comment_preproc => "noitalic #289870",
    :generic_deleted => "#c02828",
    :generic_emph => "italic",
    :generic_error => "#c02828",
    :generic_heading => "#666666",
    :generic_inserted => "#388038",
    :generic_output => "#666666",
    :generic_prompt => "#444444",
    :generic_strong => "bold",
    :generic_subheading => "#444444",
    :generic_traceback => "#2838b0"

  }

  alias Makeup.Styles.HTML.Style

  @style_struct Style.make_style(
      short_name: "lovelace",
      long_name: "Lovelace Style",
      background_color: "#ffffff",
      highlight_color: "#ffffcc",
      styles: @styles)

  def style() do
    @style_struct
  end
end