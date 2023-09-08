
defmodule Makeup.Styles.HTML.AutumnStyle do
  @moduledoc false

  @styles %{
    :error => "#F00 bg:#FAA",
    :keyword => "#0000aa",
    :keyword_type => "#00aaaa",
    :name_attribute => "#1e90ff",
    :name_builtin => "#00aaaa",
    :name_class => "underline #00aa00",
    :name_constant => "#aa0000",
    :name_decorator => "#888888",
    :name_entity => "bold #800",
    :name_function => "#00aa00",
    :name_namespace => "underline #00aaaa",
    :name_tag => "bold #1e90ff",
    :name_variable => "#aa0000",
    :string => "#aa5500",
    :string_regex => "#009999",
    :string_symbol => "#0000aa",
    :number => "#009999",
    :operator_word => "#0000aa",
    :comment => "italic #aaaaaa",
    :comment_preproc => "noitalic #4c8317",
    :comment_special => "italic #0000aa",
    :generic_deleted => "#aa0000",
    :generic_emph => "italic",
    :generic_error => "#aa0000",
    :generic_heading => "bold #000080",
    :generic_inserted => "#00aa00",
    :generic_output => "#888888",
    :generic_prompt => "#555555",
    :generic_strong => "bold",
    :generic_subheading => "bold #800080",
    :generic_traceback => "#aa0000"

  }

  alias Makeup.Styles.HTML.Style

  @style_struct Style.make_style(
      short_name: "autumn",
      long_name: "Autumn Style",
      background_color: "#ffffff",
      highlight_color: "#ffffcc",
      styles: @styles)

  def style() do
    @style_struct
  end
end