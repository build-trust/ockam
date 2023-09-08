
defmodule Makeup.Styles.HTML.DefaultStyle do
  @moduledoc false

  @styles %{
    :error => "border:#FF0000",
    :keyword => "bold #008000",
    :keyword_pseudo => "nobold",
    :keyword_type => "nobold #B00040",
    :name_attribute => "#7D9029",
    :name_builtin => "#008000",
    :name_class => "bold #0000FF",
    :name_constant => "#880000",
    :name_decorator => "#AA22FF",
    :name_entity => "bold #999999",
    :name_exception => "bold #D2413A",
    :name_function => "#0000FF",
    :name_label => "#A0A000",
    :name_namespace => "bold #0000FF",
    :name_tag => "bold #008000",
    :name_variable => "#19177C",
    :string => "#BA2121",
    :string_doc => "italic",
    :string_escape => "bold #BB6622",
    :string_interpol => "bold #BB6688",
    :string_other => "#008000",
    :string_regex => "#BB6688",
    :string_symbol => "#19177C",
    :number => "#666666",
    :operator => "#666666",
    :operator_word => "bold #AA22FF",
    :comment => "italic #408080",
    :comment_preproc => "noitalic #BC7A00",
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
      short_name: "default",
      long_name: "Default Style",
      background_color: "#f8f8f8",
      highlight_color: "#ffffcc",
      styles: @styles)

  def style() do
    @style_struct
  end
end