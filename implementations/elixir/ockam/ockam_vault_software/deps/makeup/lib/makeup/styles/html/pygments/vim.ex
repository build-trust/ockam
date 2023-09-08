
defmodule Makeup.Styles.HTML.VimStyle do
  @moduledoc false

  @styles %{
    :error => "border:#FF0000",
    :keyword => "#cdcd00",
    :keyword_declaration => "#00cd00",
    :keyword_namespace => "#cd00cd",
    :keyword_type => "#00cd00",
    :name_builtin => "#cd00cd",
    :name_class => "#00cdcd",
    :name_exception => "bold #666699",
    :name_variable => "#00cdcd",
    :string => "#cd0000",
    :number => "#cd00cd",
    :operator => "#3399cc",
    :operator_word => "#cdcd00",
    :comment => "#000080",
    :comment_special => "bold #cd0000",
    :generic_deleted => "#cd0000",
    :generic_emph => "italic",
    :generic_error => "#FF0000",
    :generic_heading => "bold #000080",
    :generic_inserted => "#00cd00",
    :generic_output => "#888",
    :generic_prompt => "bold #000080",
    :generic_strong => "bold",
    :generic_subheading => "bold #800080",
    :generic_traceback => "#04D"

  }

  alias Makeup.Styles.HTML.Style

  @style_struct Style.make_style(
      short_name: "vim",
      long_name: "Vim Style",
      background_color: "#000000",
      highlight_color: "#222222",
      styles: @styles)

  def style() do
    @style_struct
  end
end