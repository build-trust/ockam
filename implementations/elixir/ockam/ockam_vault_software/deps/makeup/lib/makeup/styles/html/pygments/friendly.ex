
defmodule Makeup.Styles.HTML.FriendlyStyle do
  @moduledoc false

  @styles %{
    :error => "border:#FF0000",
    :keyword => "bold #007020",
    :keyword_pseudo => "nobold",
    :keyword_type => "nobold #902000",
    :name_attribute => "#4070a0",
    :name_builtin => "#007020",
    :name_class => "bold #0e84b5",
    :name_constant => "#60add5",
    :name_decorator => "bold #555555",
    :name_entity => "bold #d55537",
    :name_exception => "#007020",
    :name_function => "#06287e",
    :name_label => "bold #002070",
    :name_namespace => "bold #0e84b5",
    :name_tag => "bold #062873",
    :name_variable => "#bb60d5",
    :string => "#4070a0",
    :string_doc => "italic",
    :string_escape => "bold #4070a0",
    :string_interpol => "italic #70a0d0",
    :string_other => "#c65d09",
    :string_regex => "#235388",
    :string_symbol => "#517918",
    :number => "#40a070",
    :operator => "#666666",
    :operator_word => "bold #007020",
    :comment => "italic #60a0b0",
    :comment_preproc => "noitalic #007020",
    :comment_special => "noitalic bg:#fff0f0",
    :generic_deleted => "#A00000",
    :generic_emph => "italic",
    :generic_error => "#FF0000",
    :generic_heading => "bold #000080",
    :generic_inserted => "#00A000",
    :generic_output => "#888",
    :generic_prompt => "bold #c65d09",
    :generic_strong => "bold",
    :generic_subheading => "bold #800080",
    :generic_traceback => "#04D"

  }

  alias Makeup.Styles.HTML.Style

  @style_struct Style.make_style(
      short_name: "friendly",
      long_name: "Friendly Style",
      background_color: "#f0f0f0",
      highlight_color: "#ffffcc",
      styles: @styles)

  def style() do
    @style_struct
  end
end