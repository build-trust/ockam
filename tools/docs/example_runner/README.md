# example_runner

Utility for automated running of Ockam examples. Test scenarios are described in `.ron` files
using a small DSL, divided into stages of execution.

## RON scripts

The RON scripts consist of a name, and array of stages to execute. Stages run sequentially
from top to bottom. Each stage consists of a series of steps.

Steps can contain various commands:

- By default, a value will be interpreted as an executable file name to run.
  - Programs listed as steps _will be run concurrently_ but started in the order defined.
- Several special keywords exist to alter or check the execution of an example:
  - sleep N: sleeps for N seconds
  - match S: matches a string, up to space or newline, beginning with substring S
  - out N: emit the string matched by the Nth match statement to stdout
  - quit: terminate all programs running in the stage

## Matching and out commands

Probably the most complex commands are match and out. Match statements form a stack of matches
that can be referenced by later out commands. This stack is zero indexed. An example of using
this feature can be seen in `kafka.ron`:

```ron
"match alice_to_bob_", "match bob_to_alice_",
"out 1",
"out 0",
```

This will match `alice_to_bob_foo` followed by `bob_to_alice_bar`. It will then emit on stdout
in reverse order: `bob_to_alice_bar` and then `alice_to_bob_foo`.
