We need FetchContent to download external dependencies in the various
subdirectories.

FetchContent was added in CMake version 3.11, for versions lower than that
we include a vendored copy of the module's code from CMake v3.11.3
