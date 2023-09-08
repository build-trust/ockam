
defmodule Makeup.Styles.HTML.FruityStyle do
  @moduledoc false

  @styles %{
    :keyword => "#fb660a bold",
    :keyword_pseudo => "nobold",
    :keyword_type => "#cdcaa9 bold",
    :name_attribute => "#ff0086 bold",
    :name_constant => "#0086d2",
    :name_function => "#ff0086 bold",
    :name_tag => "#fb660a bold",
    :name_variable => "#fb660a",
    :string => "#0086d2",
    :number => "#0086f7 bold",
    :comment => "#008800 bg:#0f140f italic",
    :comment_preproc => "#ff0007 bold",
    :generic_heading => "#ffffff bold",
    :generic_output => "#444444 bg:#222222",
    :generic_subheading => "#ffffff bold",

  }

  alias Makeup.Styles.HTML.Style

  @style_struct Style.make_style(
      short_name: "fruity",
      long_name: "Fruity Style",
      background_color: "#111111",
      highlight_color: "#333333",
      styles: @styles)

  def style() do
    @style_struct
  end
end