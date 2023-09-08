
defmodule Makeup.Styles.HTML.RrtStyle do
  @moduledoc false

  @styles %{
    :keyword => "#ff0000",
    :keyword_type => "#ee82ee",
    :name_constant => "#7fffd4",
    :name_function => "#ffff00",
    :name_variable => "#eedd82",
    :string => "#87ceeb",
    :comment => "#00ff00",
    :comment_preproc => "#e5e5e5",

  }

  alias Makeup.Styles.HTML.Style

  @style_struct Style.make_style(
      short_name: "rrt",
      long_name: "Rrt Style",
      background_color: "#000000",
      highlight_color: "#0000ff",
      styles: @styles)

  def style() do
    @style_struct
  end
end