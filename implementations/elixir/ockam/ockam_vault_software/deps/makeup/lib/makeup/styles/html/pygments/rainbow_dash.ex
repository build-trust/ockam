
defmodule Makeup.Styles.HTML.RainbowDashStyle do
  @moduledoc false

  @styles %{
    :text => "#4d4d4d",
    :error => "bg:#cc0000 #ffffff",
    :keyword => "bold #2c5dcd",
    :keyword_pseudo => "nobold",
    :keyword_type => "#5918bb",
    :name_attribute => "italic #2c5dcd",
    :name_builtin => "bold #5918bb",
    :name_class => "underline",
    :name_constant => "#318495",
    :name_decorator => "bold #ff8000",
    :name_entity => "bold #5918bb",
    :name_exception => "bold #5918bb",
    :name_function => "bold #ff8000",
    :name_tag => "bold #2c5dcd",
    :string => "#00cc66",
    :string_doc => "italic",
    :string_escape => "bold #c5060b",
    :string_other => "#318495",
    :string_symbol => "bold #c5060b",
    :number => "bold #5918bb",
    :operator => "#2c5dcd",
    :operator_word => "bold",
    :comment => "italic #0080ff",
    :comment_preproc => "noitalic",
    :comment_special => "bold",
    :generic_deleted => "border:#c5060b bg:#ffcccc",
    :generic_emph => "italic",
    :generic_error => "#ff0000",
    :generic_heading => "bold #2c5dcd",
    :generic_inserted => "border:#00cc00 bg:#ccffcc",
    :generic_output => "#aaaaaa",
    :generic_prompt => "bold #2c5dcd",
    :generic_strong => "bold",
    :generic_subheading => "bold #2c5dcd",
    :generic_traceback => "#c5060b"

  }

  alias Makeup.Styles.HTML.Style

  @style_struct Style.make_style(
      short_name: "rainbow_dash",
      long_name: "RainbowDash Style",
      background_color: "#ffffff",
      highlight_color: "#ffffcc",
      styles: @styles)

  def style() do
    @style_struct
  end
end