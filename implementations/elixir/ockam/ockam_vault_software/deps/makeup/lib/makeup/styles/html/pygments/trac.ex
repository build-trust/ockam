
defmodule Makeup.Styles.HTML.TracStyle do
  @moduledoc false

  @styles %{
    :error => "bg:#e3d2d2 #a61717",
    :keyword => "bold",
    :keyword_type => "#445588",
    :name_attribute => "#008080",
    :name_builtin => "#999999",
    :name_class => "bold #445588",
    :name_constant => "#008080",
    :name_entity => "#800080",
    :name_exception => "bold #990000",
    :name_function => "bold #990000",
    :name_namespace => "#555555",
    :name_tag => "#000080",
    :name_variable => "#008080",
    :string => "#bb8844",
    :string_regex => "#808000",
    :number => "#009999",
    :operator => "bold",
    :comment => "italic #999988",
    :comment_preproc => "bold noitalic #999999",
    :comment_special => "bold #999999",
    :generic_deleted => "bg:#ffdddd #000000",
    :generic_emph => "italic",
    :generic_error => "#aa0000",
    :generic_heading => "#999999",
    :generic_inserted => "bg:#ddffdd #000000",
    :generic_output => "#888888",
    :generic_prompt => "#555555",
    :generic_strong => "bold",
    :generic_subheading => "#aaaaaa",
    :generic_traceback => "#aa0000"

  }

  alias Makeup.Styles.HTML.Style

  @style_struct Style.make_style(
      short_name: "trac",
      long_name: "Trac Style",
      background_color: "#ffffff",
      highlight_color: "#ffffcc",
      styles: @styles)

  def style() do
    @style_struct
  end
end