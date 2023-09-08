
defmodule Makeup.Styles.HTML.ParaisoLightStyle do
  @moduledoc false

  @styles %{
    :text => "#2f1e2e",
    :error => "#ef6155",
    :keyword => "#815ba4",
    :keyword_namespace => "#5bc4bf",
    :keyword_type => "#fec418",
    :name => "#2f1e2e",
    :name_attribute => "#06b6ef",
    :name_class => "#fec418",
    :name_constant => "#ef6155",
    :name_decorator => "#5bc4bf",
    :name_exception => "#ef6155",
    :name_function => "#06b6ef",
    :name_namespace => "#fec418",
    :name_other => "#06b6ef",
    :name_tag => "#5bc4bf",
    :name_variable => "#ef6155",
    :literal => "#f99b15",
    :string => "#48b685",
    :string_char => "#2f1e2e",
    :string_doc => "#8d8687",
    :string_escape => "#f99b15",
    :string_interpol => "#f99b15",
    :number => "#f99b15",
    :operator => "#5bc4bf",
    :punctuation => "#2f1e2e",
    :comment => "#8d8687",
    :generic_deleted => "#ef6155",
    :generic_emph => "italic",
    :generic_heading => "bold #2f1e2e",
    :generic_inserted => "#48b685",
    :generic_prompt => "bold #8d8687",
    :generic_strong => "bold",
    :generic_subheading => "bold #5bc4bf",

  }

  alias Makeup.Styles.HTML.Style

  @style_struct Style.make_style(
      short_name: "paraiso_light",
      long_name: "ParaisoLight Style",
      background_color: "#e7e9db",
      highlight_color: "#a39e9b",
      styles: @styles)

  def style() do
    @style_struct
  end
end