
defmodule Makeup.Styles.HTML.VisualStudioStyle do
  @moduledoc false

  @styles %{
    :error => "border:#FF0000",
    :keyword => "#0000ff",
    :keyword_type => "#2b91af",
    :name_class => "#2b91af",
    :string => "#a31515",
    :operator_word => "#0000ff",
    :comment => "#008000",
    :comment_preproc => "#0000ff",
    :generic_emph => "italic",
    :generic_heading => "bold",
    :generic_prompt => "bold",
    :generic_strong => "bold",
    :generic_subheading => "bold",

  }

  alias Makeup.Styles.HTML.Style

  @style_struct Style.make_style(
      short_name: "vs",
      long_name: "VisualStudio Style",
      background_color: "#ffffff",
      highlight_color: "#ffffcc",
      styles: @styles)

  def style() do
    @style_struct
  end
end