
defmodule Makeup.Styles.HTML.PastieStyle do
  @moduledoc false

  @styles %{
    :error => "bg:#e3d2d2 #a61717",
    :keyword => "bold #008800",
    :keyword_pseudo => "nobold",
    :keyword_type => "#888888",
    :name_attribute => "#336699",
    :name_builtin => "#003388",
    :name_class => "bold #bb0066",
    :name_constant => "bold #003366",
    :name_decorator => "#555555",
    :name_exception => "bold #bb0066",
    :name_function => "bold #0066bb",
    :name_property => "bold #336699",
    :name_label => "italic #336699",
    :name_namespace => "bold #bb0066",
    :name_tag => "bold #bb0066",
    :name_variable => "#336699",
    :name_variable_class => "#336699",
    :name_variable_global => "#dd7700",
    :name_variable_instance => "#3333bb",
    :string => "bg:#fff0f0 #dd2200",
    :string_escape => "#0044dd",
    :string_interpol => "#3333bb",
    :string_other => "bg:#f0fff0 #22bb22",
    :string_regex => "bg:#fff0ff #008800",
    :string_symbol => "#aa6600",
    :number => "bold #0000DD",
    :operator_word => "#008800",
    :comment => "#888888",
    :comment_preproc => "bold #cc0000",
    :comment_special => "bg:#fff0f0 bold #cc0000",
    :generic_deleted => "bg:#ffdddd #000000",
    :generic_emph => "italic",
    :generic_error => "#aa0000",
    :generic_heading => "#333",
    :generic_inserted => "bg:#ddffdd #000000",
    :generic_output => "#888888",
    :generic_prompt => "#555555",
    :generic_strong => "bold",
    :generic_subheading => "#666",
    :generic_traceback => "#aa0000"

  }

  alias Makeup.Styles.HTML.Style

  @style_struct Style.make_style(
      short_name: "pastie",
      long_name: "Pastie Style",
      background_color: "#ffffff",
      highlight_color: "#ffffcc",
      styles: @styles)

  def style() do
    @style_struct
  end
end