# example_blocks

This utility will embed example source from `examples` into code blocks in Markdown files.
This is used as a step to ensure that the source in guide code blocks matches exactly to
example source.

## Checking Guides

The `EXAMPLES_DIR` environment variable should be set to the absolute path of the `examples`
directory of the ockam repository.

Pass the file to be checked as an argument to the utility. It will search for any code
blocks that begin with a comment of the form `// examples/foo.rs` and then print that source.
