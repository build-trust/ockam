# Ockam Desktop Application


The Ockam Desktop Application is a native MacOS application that provides a GUI for users to interact with Ockam.
It's built on top of on the crate `ockam_app_lib` which exposes C APIs for interacting with Ockam.

## How to build

To build the Ockam Desktop Application:
1. Build `ockam_app_lib` by executing `cargo build -p ockam_app_lib`
2. After this point, you have two options:

   2a. Open the Xcode project `Ockam.xcodeproj` and build the project from Xcode by pressing `Product => Build`

   2b. Compile from command line via `xcodebuild -project Ockam.xcodeproj/ -scheme Ockam -configuration Debug -derivedDataPath /tmp/build/`
