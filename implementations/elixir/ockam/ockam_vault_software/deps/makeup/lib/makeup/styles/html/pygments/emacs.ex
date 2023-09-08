
defmodule Makeup.Styles.HTML.EmacsStyle do
  @moduledoc false

  @styles %{
    :error => "border:#FF0000",
    :keyword => "bold #AA22FF",
    :keyword_pseudo => "nobold",
    :keyword_type => "bold #00BB00",
    :name_attribute => "#BB4444",
    :name_builtin => "#AA22FF",
    :name_class => "#0000FF",
    :name_constant => "#880000",
    :name_decorator => "#AA22FF",
    :name_entity => "bold #999999",
    :name_exception => "bold #D2413A",
    :name_function => "#00A000",
    :name_label => "#A0A000",
    :name_namespace => "bold #0000FF",
    :name_tag => "bold #008000",
    :name_variable => "#B8860B",
    :string => "#BB4444",
    :string_doc => "italic",
    :string_escape => "bold #BB6622",
    :string_interpol => "bold #BB6688",
    :string_other => "#008000",
    :string_regex => "#BB6688",
    :string_symbol => "#B8860B",
    :number => "#666666",
    :operator => "#666666",
    :operator_word => "bold #AA22FF",
    :comment => "italic #008800",
    :comment_preproc => "noitalic",
    :comment_special => "noitalic bold",
    :generic_deleted => "#A00000",
    :generic_emph => "italic",
    :generic_error => "#FF0000",
    :generic_heading => "bold #000080",
    :generic_inserted => "#00A000",
    :generic_output => "#888",
    :generic_prompt => "bold #000080",
    :generic_strong => "bold",
    :generic_subheading => "bold #800080",
    :generic_traceback => "#04D"

  }

  alias Makeup.Styles.HTML.Style

  @style_struct Style.make_style(
      short_name: "emacs",
      long_name: "Emacs Style",
      background_color: "#f8f8f8",
      highlight_color: "#ffffcc",
      styles: @styles)

  def style() do
    @style_struct
  end
end