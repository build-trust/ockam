defmodule Makeup.Token.Utils do
  @moduledoc false
  alias Makeup.Token.Utils.Hierarchy

  @hierarchy [
    {:text, nil},
    {:whitespace, "w"},
    {:escape, "esc"},
    {:error, "err"},
    {:other, "x"},

    {:comment, "c", [
      {:comment_hashbang, "ch"},
      {:comment_multiline, "cm"},
      {:comment_preproc, "cp", [
        {:comment_preproc_file, "cpf"}]},
      {:comment_single, "c1"},
      {:comment_special, "cs"}]},

    {:keyword, "k", [
      {:keyword_constant, "kc"},
      {:keyword_declaration, "kd"},
      {:keyword_namespace, "kn"},
      {:keyword_pseudo, "kp"},
      {:keyword_reserved, "kr"},
      {:keyword_type, "kt"}]},

    {:literal, "l", [
      {:literal_date, "ld"}]},

    {:name, "n", [
      {:name_attribute, "na"},
      {:name_builtin, "nb", [
        {:name_builtin_pseudo, "bp"}]},
      {:name_class, "nc"},
      {:name_constant, "no"},
      {:name_decorator, "nd"},
      {:name_entity, "ni"},
      {:name_exception, "ne"},
      {:name_function, "nf", [
        {:name_function_magic, "fm"}]},
      {:name_property, "py"},
      {:name_label, "nl"},
      {:name_namespace, "nn"},
      {:name_other, "nx"},
      {:name_tag, "nt"},
      {:name_variable, "nv", [
        {:name_variable_class, "vc"},
        {:name_variable_global, "vg"},
        {:name_variable_instance, "vi"},
        {:name_variable_magic, "vm"}]}]},

    {:number, "m", [
      {:number_bin, "mb"},
      {:number_float, "mf"},
      {:number_hex, "mh"},
      {:number_integer, "mi", [
        {:number_integer_long, "il"}]},
      {:number_oct, "mo"}]},

    {:string, "s", [
      {:string_affix, "sa"},
      {:string_backtick, "sb"},
      {:string_char, "sc"},
      {:string_delimiter, "dl"},
      {:string_doc, "sd"},
      {:string_double, "s2"},
      {:string_escape, "se"},
      {:string_heredoc, "sh"},
      {:string_interpol, "si"},
      {:string_other, "sx"},
      {:string_regex, "sr"},
      {:string_sigil, "sx"},
      {:string_single, "s1"},
      {:string_symbol, "ss"}]},

    {:operator, "o", [
      {:operator_word, "ow"}]},

    {:punctuation, "p"},

    {:generic, "g", [
      {:generic_deleted, "gd"},
      {:generic_emph, "ge"},
      {:generic_error, "gr"},
      {:generic_heading, "gh"},
      {:generic_inserted, "gi"},
      {:generic_prompt, "gp"},
      {:generic_output, "go"},
      {:generic_strong, "gs"},
      {:generic_subheading, "gu"},
      {:generic_traceback, "gt"}]}
  ]


  @precedence Hierarchy.hierarchy_to_precedence(@hierarchy)
  @token_to_class_map Hierarchy.style_to_class_map(@hierarchy)
  @standard_token_types Map.keys(@token_to_class_map)

  def precedence do
    @precedence
  end

  def token_to_class_map do
    @token_to_class_map
  end

  def standard_token_types do
    @standard_token_types
  end

  def css_class_for_token_type(token_type) do
    Map.get(@token_to_class_map, token_type, nil)
  end
end
