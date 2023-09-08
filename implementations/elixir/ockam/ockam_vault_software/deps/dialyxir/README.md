# Dialyxir

[![Build Status](https://travis-ci.org/jeremyjh/dialyxir.svg?branch=master)](https://travis-ci.org/jeremyjh/dialyxir)
[![Module Version](https://img.shields.io/hexpm/v/dialyxir.svg)](https://hex.pm/packages/dialyxir)
[![Hex Docs](https://img.shields.io/badge/hex-docs-lightgreen.svg)](https://hexdocs.pm/dialyxir/)
[![Total Download](https://img.shields.io/hexpm/dt/dialyxir.svg)](https://hex.pm/packages/dialyxir)
[![License](https://img.shields.io/hexpm/l/dialyxir.svg)](https://github.com/jeremyjh/dialyxir/blob/master/LICENSE)
[![Last Updated](https://img.shields.io/github/last-commit/jeremyjh/dialyxir.svg)](https://github.com/jeremyjh/dialyxir/commits/master)

Mix tasks to simplify use of Dialyzer in Elixir projects.

## Changes in 1.0

Elixir 1.6 is required, to support the new pretty printing feature. If your
project is not yet on 1.6, continue to specify 0.5 in your mix deps.

Warning messages have been greatly improved, but are filtered through the legacy formatter to support your existing ignore files. You can optionally use the new Elixir [term format](#elixir-term-format) for ignore files. You may want to use the `--format short` argument in your CI pipelines. There are several formats, also there is a new `explain` feature - for details see CLI [options](#command-line-options).

## Quickstart
If you are planning to use Dialyzer with an application built with the [Phoenix Framework](http://www.phoenixframework.org/), check out the [Quickstart wiki](https://github.com/jeremyjh/dialyxir/wiki/Phoenix-Dialyxir-Quickstart).

## Installation

Dialyxir is available on [hex.pm](https://hex.pm/packages/dialyxir).

You can either add it as a dependency in your mix.exs, or install it globally as an archive task.

To add it to a mix project, just add a line like this in your deps function in mix.exs:

```elixir
defp deps do
  [
    {:dialyxir, "~> 1.0", only: [:dev], runtime: false},
  ]
end
```

```console
mix do deps.get, deps.compile
```

## Usage

Use dialyxir from the directory of the mix project you want to analyze; a PLT file will be created or updated if required and the project will be automatically compiled.

```console
mix dialyzer
```

### Command line options

  * `--no-compile`         - do not compile even if needed.
  * `--no-check`           - do not perform (quick) check to see if PLT needs to be updated.
  * `--ignore-exit-status` - display warnings but do not halt the VM or return an exit status code
  *  `--format short`      - format the warnings in a compact format.
  *  `--format raw`        - format the warnings in format returned before Dialyzer formatting
  *  `--format dialyxir`   - format the warnings in a pretty printed format
  *  `--format dialyzer`   - format the warnings in the original Dialyzer format
  *  `--quiet`             - suppress all informational messages

Warning flags passed to this task are passed on to `:dialyzer` - e.g.

```console
mix dialyzer --unmatched_returns
```

There is information available about the warnings via the explain task - e.g.

```console
mix dialyzer.explain unmatched_return
```

If invoked without arguments, `mix dialyzer.explain` will list all the known warnings.

## Continuous Integration

To use Dialyzer in CI, you must be aware of several things:

1) Building the PLT file may take a while if a project has many dependencies
2) The PLT should be cached using the CI caching system
3) The PLT will need to be rebuilt whenever adding a new Erlang or Elixir version to build matrix

Using Travis, this would look like:

`.travis.yml`
```markdown
language: elixir

elixir:
  - 1.8

otp_release:
  - 21.0

script:
  - mix dialyzer

cache:
  directories:
    - priv/plts
```

`mix.exs`
```elixir
def project do
  [
    ...
    dialyzer: [
      plt_file: {:no_warn, "priv/plts/dialyzer.plt"}
    ]
  ]
end
```

`.gitignore`
```
/priv/plts/*.plt
/priv/plts/*.plt.hash
```

## With Explaining Stuff

[Dialyzer](http://www.erlang.org/doc/apps/dialyzer/dialyzer_chapter.html) is a static analysis tool for Erlang and other languages that compile to BEAM bytecode for the Erlang VM. It can analyze the BEAM files and provide warnings about problems in your code including type mismatches and other issues that are commonly detected by static language compilers. The analysis can be improved by inclusion of type hints (called [specs](https://hexdocs.pm/elixir/typespecs.html)) but it can be useful even without those. For more information I highly recommend the [Success Typings](http://user.it.uu.se/~kostis/Papers/succ_types.pdf) paper that describes the theory behind the tool.


Usage is straightforward but you should be aware of the available configuration settings you may wish to add to your mix.exs file.

### PLT

The Persistent Lookup Table (PLT) is basically a cached output of the analysis. This is important because you'd probably stab yourself in the eye with
a fork if you had to wait for Dialyzer to analyze all the standard library and OTP modules you are using every time you ran it.
Running the mix task `dialyzer` by default builds several PLT files:

  * A core Erlang file in `$MIX_HOME/dialyxir_erlang-[OTP Version].plt`
  * A core Elixir file in `$MIX_HOME/dialyxir_erlang-[OTP Version]_elixir-[Elixir Version].plt`
  * A project environment specific file in `_build/env/dialyze_erlang-[OTP Version]_elixir-[Elixir Version]_deps-dev.plt`

The core files are simply copied to your project folder when you run `dialyxir` for the first time with a given version of Erlang and Elixir. By default, all
the modules in the project PLT are checked against your dependencies to be sure they are up to date. If you do not want to use MIX_HOME to store your core Erlang and Elixir files, you can provide a `:plt_core_path` key with a file path. You can specify a different directory for the project PLT file with the `:plt_local_path keyword`. You can specify a different filename for the project PLT file with the `:plt_file keyword` - this is deprecated because people were using it with the old `dialyxir` to have project-specific PLTs, which are now the default. To silence the deprecation warning, specify this value as `plt_file: {:no_warn, "/myproject/mypltfile"}`.

The core PLTs include a basic set of OTP applications, as well as all of the Elixir standard libraries.
The apps included by default are `[:erts, :kernel, :stdlib, :crypto]`.

If you don't want to include the default apps you can specify a `:plt_apps` key and list there only the apps you want in the PLT. Using this option will mean dependencies are not added automatically (see below). If you want to just add an application to the list of defaults and dependencies you can use the `:plt_add_apps` key.

If you want to ignore a specific dependency, you can specify it in the `:plt_ignore_apps` key.

#### Dependencies

OTP application dependencies are (transitively) added to your PLT by default. The applications added are the same as you would see displayed with the command `mix app.tree`. There is also a `:plt_add_deps` option you can set to control the dependencies added. The following options are supported:

  * `:apps_direct` - Only Direct OTP runtime application dependencies - not the entire tree
  * `:app_tree` - Transitive OTP runtime application dependencies e.g. `mix app.tree` (default)


The example below changes the default to include only direct OTP dependencies, adds another specific dependency, and removes a dependency from the list. This can be helpful if a large dependency tree is creating memory issues and only some of the transitive dependencies are required for analysis.

```elixir
def project do
  [
    app: :my_app,
    version: "0.0.1",
    deps: deps,
    dialyzer: [
      plt_add_deps: :apps_direct,
      plt_add_apps: [:wx],
      plt_ignore_apps: [:mnesia]
    ]
  ]
end
```

#### Explanations

Explanations are available for classes of warnings by executing `mix dialyzer.explain warning_name`. It will include a description about the type of warning, as well as a small example that would also cause that warning. Poor explanations and examples should be considered issues in this library, and pull requests are very welcome! The warning name is returned from the `--format short` and `--format dialyzer` flags. List available warnings with `mix dialyzer.explain`.

#### Formats

Dialyxir supports formatting the errors in several different ways:

  * Short - By passing `--format short`, the structs and other spec/type information will be dropped from the error message, with a minimal message. This is useful for CI environments. Includes `warning_name ` for use in explanations.
  * Dialyzer - By passing `--format dialyzer`, the messages will be printed in the default Dialyzer format. This format is used in [legacy string matching](#simple-string-matches) ignore files.
  * Raw - By passing `--format raw`, messages will be printed in their form before being pretty printed by Dialyzer or Dialyxir.
  * Dialyxir (default) -- By passing `--format dialyxir`, messages will be converted to Elixir style messages then pretty printed and formatted. Includes `warning_name ` for use in explanations.

### Flags

Dialyzer supports a number of warning flags used to enable or disable certain kinds of analysis features. Until version 0.4, `dialyxir` used by default the additional warning flags shown in the example below. However some of these create warnings that are often more confusing than helpful, particularly to new users of Dialyzer. As of 0.4, there are no longer any flags used by default. To get the old behavior, specify them in your Mix project file. For compatibility reasons you can use either the `-Wwarning` convention of the dialyzer CLI, or (preferred) the `WarnOpts` atoms supported by the [API](http://erlang.org/doc/man/dialyzer.html#gui-1).  e.g.

```elixir
def project do
  [
    app: :my_app,
    version: "0.0.1",
    deps: deps,
    dialyzer: [flags: ["-Wunmatched_returns", :error_handling, :race_conditions, :underspecs]]
  ]
end
```

### Paths

By default only the ebin in the `_build` directory for the current mix environment of your project is included in paths to search for BEAM files to perform analysis on. You can specify a list of locations to find BEAMS for analysis with :paths keyword.

```elixir
def project do
  [
    app: :my_app,
    version: "0.0.1",
    deps: deps,
    dialyzer: [
      plt_add_apps: [:mnesia],
      flags: [:unmatched_returns, :error_handling, :race_conditions, :no_opaque],
      paths: ["_build/dev/lib/my_app/ebin", "_build/dev/lib/foo/ebin"]
    ]
  ]
end
```

### Ignore Warnings
#### Dialyxir defaults

By default `dialyxir` has always included the `:unknown` warning option so that warnings about unknown functions are returned. This is usually a clue that the PLT is not complete and it may be best to leave it on, but it can be disabled entirely by specifying `remove_defaults: [:unknown]` in your config.

A better option is to ignore the specific warnings you can't fix (maybe due to a bug upstream, or a dependency you just don't want to include in your PLT due to time/memory in building the PLT file.)

#### Module attribute

Dialyzer has a built-in support for ignoring warnings through a `@dialyzer` module attribute. For example:

```elixir
defmodule Myapp.Repo do
  use Ecto.Repo, otp_app: :myapp
  @dialyzer {:nowarn_function, rollback: 1}
end
```

More details can be found in the [erlang documentation](http://erlang.org/doc/man/dialyzer.html#requesting-or-suppressing-warnings-in-source-files)

#### Ignore file

If you want to ignore well-known warnings, you can specify a file path in `:ignore_warnings`.

```elixir
def project do
  [
    app: :my_app,
    version: "0.0.1",
    deps: deps,
    dialyzer: [ignore_warnings: "dialyzer.ignore-warnings"]
  ]
end
```

This file comes in two formats: `--format dialyzer` string matches (compatible with `<= 0.5.1` ignore files), and the [term format](#elixir-term-format).

#### Simple String Matches

Any line of dialyzer format output (partially) matching a line in `"dialyzer.ignore-warnings"` is filtered.

Note that copying output in the default format will not work!  Run `mix dialyzer --format dialyzer` to produce output suitable for the ignore file.

For example, in a project where `mix dialyzer --format dialyzer` outputs:

```
  Proceeding with analysis...
config.ex:64: The call ets:insert('Elixir.MyApp.Config',{'Elixir.MyApp.Config',_}) might have an unintended effect due to a possible race condition caused by its combination withthe ets:lookup('Elixir.MyApp.Config','Elixir.MyApp.Config') call in config.ex on line 26
config.ex:79: Guard test is_binary(_@5::#{'__exception__':='true', '__struct__':=_, _=>_}) can never succeed
config.ex:79: Guard test is_atom(_@6::#{'__exception__':='true', '__struct__':=_, _=>_}) can never succeed
 done in 0m1.32s
done (warnings were emitted)
```

If you wanted to ignore the last two warnings about guard tests, you could add to `dialyzer.ignore-warnings`:

```
Guard test is_binary(_@5::#{'__exception__':='true', '__struct__':=_, _=>_}) can never succeed
Guard test is_atom(_@6::#{'__exception__':='true', '__struct__':=_, _=>_}) can never succeed
```

And then run `mix dialyzer` would output:

```
  Proceeding with analysis...
config.ex:64: The call ets:insert('Elixir.MyApp.Config',{'Elixir.MyApp.Config',_}) might have an unintended effect due to a possible race condition caused by its combination withthe ets:lookup('Elixir.MyApp.Config','Elixir.MyApp.Config') call in config.ex on line 26
 done in 0m1.32s
done (warnings were emitted)
```

#### Elixir Term Format

Dialyxir also recognizes an Elixir format of the ignore file. If your ignore file is an `exs` file, Dialyxir will evaluate it and process its data structure. The file looks like the following, and can match either tuple patterns or an arbitrary Regex
applied to the *short-description* (`mix dialyzer --format short`):

```elixir
# .dialyzer_ignore.exs
[
  # {short_description}
  {":0:unknown_function Function :erl_types.t_is_opaque/1/1 does not exist."},
  # {short_description, warning_type}
  {":0:unknown_function Function :erl_types.t_to_string/1 does not exist.", :unknown_function},
  # {short_description, warning_type, line}
  {":0:unknown_function Function :erl_types.t_to_string/1 does not exist.", :unknown_function, 0},
  # {file, warning_type, line}
  {"lib/dialyxir/pretty_print.ex", :no_return, 100},
  # {file, warning_type}
  {"lib/dialyxir/warning_helpers.ex", :no_return},
  # {file}
  {"lib/dialyxir/warnings/app_call.ex"},
  # regex
  ~r/my_file\.ex.*my_function.*no local return/
]
```

#### List unused Filters

As filters tend to become obsolete (either because a discrepancy was fixed, or because the location
for which a filter is needed changes), listing unused filters might be useful. This can be done by
setting the `:list_unused_filters` option to `true` in `mix.exs`. For example:

```elixir
dialyzer: [
  ignore_warnings: "ignore_test.exs",
  list_unused_filters: true
]
```

This option can also be set on the command line with `--list-unused-filters`. When used without
`--ignore-exit-status`, this option will result in an error status code.
