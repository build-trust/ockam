
defmodule Makeup.Styles.HTML.IgorStyle do
  @moduledoc false


  @styles %{
    :keyword => "#0000FF",
    :name_class => "#007575",
    :name_decorator => "#CC00A3",
    :name_function => "#C34E00",
    :string => "#009C00",
    :comment => "italic #FF0000",

  }

  alias Makeup.Styles.HTML.Style

  @style_struct Style.make_style(
      short_name: "igor",
      long_name: "Igor Style",
      background_color: "#ffffff",
      highlight_color: "#ffffcc",
      styles: @styles)

  def style() do
    @style_struct
  end
end