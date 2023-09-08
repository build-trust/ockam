
defmodule Makeup.Styles.HTML.ManniStyle do
  @moduledoc false

  @styles %{
    :error => "bg:#FFAAAA #AA0000",
    :keyword => "bold #006699",
    :keyword_pseudo => "nobold",
    :keyword_type => "#007788",
    :name_attribute => "#330099",
    :name_builtin => "#336666",
    :name_class => "bold #00AA88",
    :name_constant => "#336600",
    :name_decorator => "#9999FF",
    :name_entity => "bold #999999",
    :name_exception => "bold #CC0000",
    :name_function => "#CC00FF",
    :name_label => "#9999FF",
    :name_namespace => "bold #00CCFF",
    :name_tag => "bold #330099",
    :name_variable => "#003333",
    :string => "#CC3300",
    :string_doc => "italic",
    :string_escape => "bold #CC3300",
    :string_interpol => "#AA0000",
    :string_other => "#CC3300",
    :string_regex => "#33AAAA",
    :string_symbol => "#FFCC33",
    :number => "#FF6600",
    :operator => "#555555",
    :operator_word => "bold #000000",
    :comment => "italic #0099FF",
    :comment_preproc => "noitalic #009999",
    :comment_special => "bold",
    :generic_deleted => "border:#CC0000 bg:#FFCCCC",
    :generic_emph => "italic",
    :generic_error => "#FF0000",
    :generic_heading => "bold #003300",
    :generic_inserted => "border:#00CC00 bg:#CCFFCC",
    :generic_output => "#AAAAAA",
    :generic_prompt => "bold #000099",
    :generic_strong => "bold",
    :generic_subheading => "bold #003300",
    :generic_traceback => "#99CC66"

  }

  alias Makeup.Styles.HTML.Style

  @style_struct Style.make_style(
      short_name: "manni",
      long_name: "Manni Style",
      background_color: "#f0f3f3",
      highlight_color: "#ffffcc",
      styles: @styles)

  def style() do
    @style_struct
  end
end