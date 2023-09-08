
defmodule Makeup.Styles.HTML.BorlandStyle do
  @moduledoc false

  @styles %{
    :error => "bg:#e3d2d2 #a61717",
    :keyword => "bold #000080",
    :name_attribute => "#FF0000",
    :name_tag => "bold #000080",
    :string => "#0000FF",
    :string_char => "#800080",
    :number => "#0000FF",
    :operator_word => "bold",
    :comment => "italic #008800",
    :comment_preproc => "noitalic #008080",
    :comment_special => "noitalic bold",
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
      short_name: "borland",
      long_name: "Borland Style",
      background_color: "#ffffff",
      highlight_color: "#ffffcc",
      styles: @styles)

  def style() do
    @style_struct
  end
end