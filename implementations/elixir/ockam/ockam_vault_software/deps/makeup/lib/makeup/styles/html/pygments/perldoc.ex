
defmodule Makeup.Styles.HTML.PerldocStyle do
  @moduledoc false

  @styles %{
    :error => "bg:#e3d2d2 #a61717",
    :keyword => "#8B008B bold",
    :keyword_type => "#00688B",
    :name_attribute => "#658b00",
    :name_builtin => "#658b00",
    :name_class => "#008b45 bold",
    :name_constant => "#00688B",
    :name_decorator => "#707a7c",
    :name_exception => "#008b45 bold",
    :name_function => "#008b45",
    :name_namespace => "#008b45 underline",
    :name_tag => "#8B008B bold",
    :name_variable => "#00688B",
    :string => "#CD5555",
    :string_heredoc => "#1c7e71 italic",
    :string_other => "#cb6c20",
    :string_regex => "#1c7e71",
    :number => "#B452CD",
    :operator_word => "#8B008B",
    :comment => "#228B22",
    :comment_preproc => "#1e889b",
    :comment_special => "#8B008B bold",
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
      short_name: "perldoc",
      long_name: "Perldoc Style",
      background_color: "#eeeedd",
      highlight_color: "#ffffcc",
      styles: @styles)

  def style() do
    @style_struct
  end
end