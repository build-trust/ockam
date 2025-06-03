## Install

- https://bats-core.readthedocs.io/en/stable/
- https://github.com/ztombol/bats-docs#installation
- https://github.com/ztombol/bats-assert

MacOS:

```bash
brew tap kaos/shell
brew install bats-assert
```

Linux:

```bash
npm install -g bats bats-support bats-assert
```

## Run the tests

To run the tests involving the creation of project tickets you need to:

 - Have an environment variable `OCKAM` pointing the `ockam` binary.
 - Set the `ORCHESTRATOR_TESTS` variable to `true`.
 - Have an enrolled identity using `ockam cluster enroll`.
 - Have a zone created using `ockam zone create`.
