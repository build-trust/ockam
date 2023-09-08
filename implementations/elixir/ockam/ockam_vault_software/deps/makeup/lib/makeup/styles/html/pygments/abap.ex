
defmodule Makeup.Styles.HTML.AbapStyle do
  @moduledoc false

  @styles %{
    :error => "#F00",
    :keyword => "#00f",
    :name => "#000",
    :string => "#5a2",
    :number => "#3af",
    :operator_word => "#00f",
    :comment => "italic #888",
    :comment_special => "#888",

  }

  alias Makeup.Styles.HTML.Style

  @style_struct Style.make_style(
      short_name: "abap",
      long_name: "Abap Style",
      background_color: "#ffffff",
      highlight_color: "#ffffcc",
      styles: @styles)

  def style() do
    @style_struct
  end
end