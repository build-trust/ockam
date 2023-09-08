
defmodule Makeup.Styles.HTML.BlackWhiteStyle do
  @moduledoc false

  @styles %{
    :error => "border:#FF0000",
    :keyword => "bold",
    :keyword_pseudo => "nobold",
    :keyword_type => "nobold",
    :name_class => "bold",
    :name_entity => "bold",
    :name_exception => "bold",
    :name_namespace => "bold",
    :name_tag => "bold",
    :string => "italic",
    :string_escape => "bold",
    :string_interpol => "bold",
    :operator_word => "bold",
    :comment => "italic",
    :comment_preproc => "noitalic",
    :generic_emph => "italic",
    :generic_heading => "bold",
    :generic_prompt => "bold",
    :generic_strong => "bold",
    :generic_subheading => "bold",

  }

  alias Makeup.Styles.HTML.Style

  @style_struct Style.make_style(
      short_name: "bw",
      long_name: "BlackWhite Style",
      background_color: "#ffffff",
      highlight_color: "#ffffcc",
      styles: @styles)

  def style() do
    @style_struct
  end
end