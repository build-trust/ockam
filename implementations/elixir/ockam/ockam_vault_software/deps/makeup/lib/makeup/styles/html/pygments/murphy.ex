
defmodule Makeup.Styles.HTML.MurphyStyle do
  @moduledoc false

  @styles %{
    :error => "#F00 bg:#FAA",
    :keyword => "bold #289",
    :keyword_pseudo => "#08f",
    :keyword_type => "#66f",
    :name_attribute => "#007",
    :name_builtin => "#072",
    :name_class => "bold #e9e",
    :name_constant => "bold #5ed",
    :name_decorator => "bold #555",
    :name_entity => "#800",
    :name_exception => "bold #F00",
    :name_function => "bold #5ed",
    :name_label => "bold #970",
    :name_namespace => "bold #0e84b5",
    :name_tag => "#070",
    :name_variable => "#036",
    :name_variable_class => "#ccf",
    :name_variable_global => "#f84",
    :name_variable_instance => "#aaf",
    :string => "bg:#e0e0ff",
    :string_char => "#88F bg:",
    :string_doc => "#D42 bg:",
    :string_escape => "bold #666",
    :string_interpol => "bg:#eee",
    :string_other => "#f88",
    :string_regex => "bg:#e0e0ff #000",
    :string_symbol => "#fc8 bg:",
    :number => "bold #60E",
    :number_float => "bold #60E",
    :number_hex => "bold #058",
    :number_integer => "bold #66f",
    :number_oct => "bold #40E",
    :operator => "#333",
    :operator_word => "bold #000",
    :comment => "#666 italic",
    :comment_preproc => "#579 noitalic",
    :comment_special => "#c00 bold",
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
      short_name: "murphy",
      long_name: "Murphy Style",
      background_color: "#ffffff",
      highlight_color: "#ffffcc",
      styles: @styles)

  def style() do
    @style_struct
  end
end