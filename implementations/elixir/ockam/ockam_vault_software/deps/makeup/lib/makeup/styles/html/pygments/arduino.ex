
defmodule Makeup.Styles.HTML.ArduinoStyle do
  @moduledoc false

  @styles %{
    :error => "#a61717",
    :keyword => "#728E00",
    :keyword_constant => "#00979D",
    :keyword_pseudo => "#00979D",
    :keyword_reserved => "#00979D",
    :keyword_type => "#00979D",
    :name => "#434f54",
    :name_builtin => "#728E00",
    :name_function => "#D35400",
    :name_other => "#728E00",
    :string => "#7F8C8D",
    :number => "#8A7B52",
    :operator => "#728E00",
    :comment => "#95a5a6",
    :comment_preproc => "#728E00",

  }

  alias Makeup.Styles.HTML.Style

  @style_struct Style.make_style(
      short_name: "arduino",
      long_name: "Arduino Style",
      background_color: "#ffffff",
      highlight_color: "#ffffcc",
      styles: @styles)

  def style() do
    @style_struct
  end
end