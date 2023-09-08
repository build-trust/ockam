# Used by "mix format"
[
  inputs: ["mix.exs", "{config,lib,test}/**/*.{ex,exs}"],
  # don't add parens around assert_value arguments
  import_deps: [:assert_value]
]
