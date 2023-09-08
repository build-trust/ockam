
defmodule Makeup.Styles.HTML.ColorfulStyle do
  @moduledoc false

  @styles %{
    :error => "#F00 bg:#FAA",
    :keyword => "bold #080",
    :keyword_pseudo => "#038",
    :keyword_type => "#339",
    :name_attribute => "#00C",
    :name_builtin => "#007020",
    :name_class => "bold #B06",
    :name_constant => "bold #036",
    :name_decorator => "bold #555",
    :name_entity => "bold #800",
    :name_exception => "bold #F00",
    :name_function => "bold #06B",
    :name_label => "bold #970",
    :name_namespace => "bold #0e84b5",
    :name_tag => "#070",
    :name_variable => "#963",
    :name_variable_class => "#369",
    :name_variable_global => "bold #d70",
    :name_variable_instance => "#33B",
    :string => "bg:#fff0f0",
    :string_char => "#04D bg:",
    :string_doc => "#D42 bg:",
    :string_escape => "bold #666",
    :string_interpol => "bg:#eee",
    :string_other => "#D20",
    :string_regex => "bg:#fff0ff #000",
    :string_symbol => "#A60 bg:",
    :number => "bold #60E",
    :number_float => "bold #60E",
    :number_hex => "bold #058",
    :number_integer => "bold #00D",
    :number_oct => "bold #40E",
    :operator => "#333",
    :operator_word => "bold #000",
    :comment => "#888",
    :comment_preproc => "#579",
    :comment_special => "bold #cc0000",
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
      short_name: "colorful",
      long_name: "Colorful Style",
      background_color: "#ffffff",
      highlight_color: "#ffffcc",
      styles: @styles)

  def style() do
    @style_struct
  end
end