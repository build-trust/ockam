
defmodule Makeup.Styles.HTML.NativeStyle do
  @moduledoc false

  @styles %{
    :error => "bg:#e3d2d2 #a61717",
    :keyword => "bold #6ab825",
    :keyword_pseudo => "nobold",
    :name_attribute => "#bbbbbb",
    :name_builtin => "#24909d",
    :name_class => "underline #447fcf",
    :name_constant => "#40ffff",
    :name_decorator => "#ffa500",
    :name_exception => "#bbbbbb",
    :name_function => "#447fcf",
    :name_namespace => "underline #447fcf",
    :name_tag => "bold #6ab825",
    :name_variable => "#40ffff",
    :string => "#ed9d13",
    :string_other => "#ffa500",
    :number => "#3677a9",
    :operator_word => "bold #6ab825",
    :comment => "italic #999999",
    :comment_preproc => "noitalic bold #cd2828",
    :comment_special => "noitalic bold #e50808 bg:#520000",
    :generic_deleted => "#d22323",
    :generic_emph => "italic",
    :generic_error => "#d22323",
    :generic_heading => "bold #ffffff",
    :generic_inserted => "#589819",
    :generic_output => "#cccccc",
    :generic_prompt => "#aaaaaa",
    :generic_strong => "bold",
    :generic_subheading => "underline #ffffff",
    :generic_traceback => "#d22323"

  }

  alias Makeup.Styles.HTML.Style

  @style_struct Style.make_style(
      short_name: "native",
      long_name: "Native Style",
      background_color: "#202020",
      highlight_color: "#404040",
      styles: @styles)

  def style() do
    @style_struct
  end
end