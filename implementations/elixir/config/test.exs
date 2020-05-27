import Config

# Disable logging during test.
#
# If we don't disable we get a lot of info log messages during tests for success
# and a lot of error log messages during tests for failure.
config :logger,
  backends: []
