defmodule EarmarkParser.Helpers.PureLinkHelpers do
  @moduledoc false

  import EarmarkParser.Helpers.StringHelpers, only: [betail: 2]
  import EarmarkParser.Helpers.AstHelpers, only: [render_link: 2]

  @pure_link_rgx ~r{\A(\s*)(\()?(https?://[[:alnum:]"'*@:+-_{\}()/.%\#]*)}u
  def convert_pure_link(src) do
    case Regex.run(@pure_link_rgx, src) do
      [_match, spaces, "", link_text] -> reparse_link(spaces, link_text)
      [_match, spaces, _, link_text]  -> remove_trailing_closing_parens(spaces, link_text)
      _ -> nil
      end
  end

  defp determine_ending_parens_by_count(leading_spaces, prefix, surplus_on_closing_parens) do
    graphemes = String.graphemes(prefix)
    open_parens_count = Enum.count(graphemes, &(&1 == "("))
    close_parens_count = Enum.count(graphemes, &(&1 == ")"))
    delta = open_parens_count - close_parens_count
    take = min(delta, surplus_on_closing_parens)
    needed =
    :lists.duplicate(max(0, take), ")")
    |> Enum.join
    link = link(prefix <> needed)
    ast =
      case leading_spaces do
        "" -> link
        _ -> [leading_spaces, link]
      end
    {ast, String.length(prefix) + String.length(leading_spaces) + max(0,take)}
  end

  @split_at_ending_parens ~r{(.*?)(\)*)\z}
  defp remove_trailing_closing_parens(leading_spaces, link_text) do
    [_, _prefix, suffix] = Regex.run(@split_at_ending_parens, link_text)
    case suffix do
      "" -> {"(", String.length(leading_spaces) + 1}
      _  -> case convert_pure_link(betail(link_text, 1)) do
        {link, length} -> {["(", link, ")"], length + 2}
        _ -> nil
      end
    end
  end

  defp reparse_link(leading_spaces, link_text) do
    [_, prefix, suffix] = Regex.run(@split_at_ending_parens, link_text)
    nof_closing_parens = String.length(suffix)
    determine_ending_parens_by_count(leading_spaces, prefix, nof_closing_parens)
  end

  defp link(text), do: render_link(text, text)

end
