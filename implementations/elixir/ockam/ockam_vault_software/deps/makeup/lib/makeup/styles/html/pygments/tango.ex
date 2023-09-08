defmodule Makeup.Styles.HTML.TangoStyle do
  @moduledoc false

  @styles %{
    :error => "#a40000 border:#ef2929",
    :other => "#000000",
    :keyword => "bold #204a87",
    :keyword_constant => "bold #204a87",
    :keyword_declaration => "bold #204a87",
    :keyword_namespace => "bold #204a87",
    :keyword_pseudo => "bold #204a87",
    :keyword_reserved => "bold #204a87",
    :keyword_type => "bold #204a87",
    :name => "#000000",
    :name_attribute => "#c4a000",
    :name_builtin => "#204a87",
    :name_builtin_pseudo => "#3465a4",
    :name_class => "#000000",
    :name_constant => "#000000",
    :name_decorator => "bold #5c35cc",
    :name_entity => "#ce5c00",
    :name_exception => "bold #cc0000",
    :name_function => "#000000",
    :name_property => "#000000",
    :name_label => "#f57900",
    :name_namespace => "#000000",
    :name_other => "#000000",
    :name_tag => "bold #204a87",
    :name_variable => "#000000",
    :name_variable_class => "#000000",
    :name_variable_global => "#000000",
    :name_variable_instance => "#000000",
    :literal => "#000000",
    :string => "#4e9a06",
    :string_backtick => "#4e9a06",
    :string_char => "#4e9a06",
    :string_doc => "italic #8f5902",
    :string_double => "#4e9a06",
    :string_escape => "#4e9a06",
    :string_heredoc => "#4e9a06",
    :string_interpol => "#4e9a06",
    :string_other => "#4e9a06",
    :string_regex => "#4e9a06",
    :string_single => "#4e9a06",
    :string_symbol => "#4e9a06",
    :number => "bold #0000cf",
    :number_float => "bold #0000cf",
    :number_hex => "bold #0000cf",
    :number_integer => "bold #0000cf",
    :number_integer_long => "bold #0000cf",
    :number_oct => "bold #0000cf",
    :operator => "bold #ce5c00",
    :operator_word => "bold #204a87",
    :punctuation => "bold #000000",
    :comment => "italic #8f5902",
    :comment_multiline => "italic #8f5902",
    :comment_preproc => "italic #8f5902",
    :comment_single => "italic #8f5902",
    :comment_special => "italic #8f5902",
    :generic => "#000000",
    :generic_deleted => "#a40000",
    :generic_emph => "italic #000000",
    :generic_error => "#ef2929",
    :generic_heading => "bold #000080",
    :generic_inserted => "#00A000",
    :generic_output => "italic #000000",
    :generic_prompt => "#8f5902",
    :generic_strong => "bold #000000",
    :generic_subheading => "bold #800080",
    :generic_traceback => "bold #a40000"

  }

  alias Makeup.Styles.HTML.Style

  @style_struct Style.make_style(
      short_name: "tango",
      long_name: "Tango Style",
      background_color: "#f8f8f8",
      highlight_color: "#ffffcc",
      styles: @styles)

  def style() do
    @style_struct
  end
end