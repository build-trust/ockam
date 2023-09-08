
defmodule Makeup.Styles.HTML.AlgolStyle do
  @moduledoc false

  @styles %{
    :error => "border:#FF0000",
    :keyword => "underline bold",
    :keyword_declaration => "italic",
    :name_builtin => "bold italic",
    :name_builtin_pseudo => "bold italic",
    :name_class => "bold italic #666",
    :name_constant => "bold italic #666",
    :name_function => "bold italic #666",
    :name_namespace => "bold italic #666",
    :name_variable => "bold italic #666",
    :string => "italic #666",
    :operator_word => "bold",
    :comment => "italic #888",
    :comment_preproc => "bold noitalic #888",
    :comment_special => "bold noitalic #888",

  }

  alias Makeup.Styles.HTML.Style

  @style_struct Style.make_style(
      short_name: "algol",
      long_name: "Algol Style",
      background_color: "#ffffff",
      highlight_color: "#ffffcc",
      styles: @styles)

  def style() do
    @style_struct
  end
end