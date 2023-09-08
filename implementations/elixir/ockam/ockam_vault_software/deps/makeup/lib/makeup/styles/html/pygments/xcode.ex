
defmodule Makeup.Styles.HTML.XcodeStyle do
  @moduledoc false

  @styles %{
    :error => "#000000",
    :keyword => "#A90D91",
    :name => "#000000",
    :name_attribute => "#836C28",
    :name_builtin => "#A90D91",
    :name_builtin_pseudo => "#5B269A",
    :name_class => "#3F6E75",
    :name_decorator => "#000000",
    :name_function => "#000000",
    :name_label => "#000000",
    :name_tag => "#000000",
    :name_variable => "#000000",
    :literal => "#1C01CE",
    :string => "#C41A16",
    :string_char => "#2300CE",
    :number => "#1C01CE",
    :operator => "#000000",
    :comment => "#177500",
    :comment_preproc => "#633820",

  }

  alias Makeup.Styles.HTML.Style

  @style_struct Style.make_style(
      short_name: "xcode",
      long_name: "Xcode Style",
      background_color: "#ffffff",
      highlight_color: "#ffffcc",
      styles: @styles)

  def style() do
    @style_struct
  end
end