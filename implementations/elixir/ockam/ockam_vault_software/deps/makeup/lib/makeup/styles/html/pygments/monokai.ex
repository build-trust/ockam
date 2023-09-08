
defmodule Makeup.Styles.HTML.MonokaiStyle do
  @moduledoc false

  @styles %{
    :text => "#f8f8f2",
    :error => "#960050 bg:#1e0010",
    :keyword => "#66d9ef",
    :keyword_namespace => "#f92672",
    :name => "#f8f8f2",
    :name_attribute => "#a6e22e",
    :name_class => "#a6e22e",
    :name_constant => "#66d9ef",
    :name_decorator => "#a6e22e",
    :name_exception => "#a6e22e",
    :name_function => "#a6e22e",
    :name_other => "#a6e22e",
    :name_tag => "#f92672",
    :literal => "#ae81ff",
    :string => "#e6db74",
    :string_escape => "#ae81ff",
    :number => "#ae81ff",
    :operator => "#f92672",
    :punctuation => "#f8f8f2",
    :comment => "#75715e",
    :generic_deleted => "#f92672",
    :generic_emph => "italic",
    :generic_inserted => "#a6e22e",
    :generic_strong => "bold",
    :generic_subheading => "#75715e",

  }

  alias Makeup.Styles.HTML.Style

  @style_struct Style.make_style(
      short_name: "monokai",
      long_name: "Monokai Style",
      background_color: "#272822",
      highlight_color: "#49483e",
      styles: @styles)

  def style() do
    @style_struct
  end
end