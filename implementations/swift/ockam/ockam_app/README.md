# Ockam Desktop Application


The Ockam Desktop Application is a native MacOS application that provides a GUI for users to interact with Ockam.
It's built on top of on the crate `ockam_app_lib` which exposes C APIs for interacting with Ockam.

## How to build

To build the Ockam Desktop Application:
1. Run `make swift_build` from the root of the repository.
2. Launch it using `./implementations/swift/build/Ockam.xcarchive/Products/Applications/Ockam.app/Contents/MacOS/Ockam`

To build the package you can use `make swift_package`.
