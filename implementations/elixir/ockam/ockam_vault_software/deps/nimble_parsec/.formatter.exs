# Used by "mix format"
locals_without_parens = [
  defparsec: 2,
  defparsec: 3,
  defparsecp: 2,
  defparsecp: 3,
  defcombinator: 2,
  defcombinator: 3,
  defcombinatorp: 2,
  defcombinatorp: 3
]

[
  inputs: ["mix.exs", "{examples,lib,test}/**/*.{ex,exs}"],
  locals_without_parens: locals_without_parens,
  export: [locals_without_parens: locals_without_parens]
]
