ROOT_DIR := $(realpath $(dir $(firstword $(MAKEFILE_LIST)))/../..)

# delete the swift build artifacts
clean:
	rm -rf "build/"

# delete the swift build artifacts and the ockam command + ockam_app_lib artifacts
very_clean: clean
	$(MAKE) -f ../rust/Makefile clean

# build the ockam command line application + ockam_app_lib crate in Debug mode
build_ockam:
	$(MAKE) -f ../rust/Makefile build

# build the ockam command line application + ockam_app_lib crate in Release mode
build_ockam_release:
	$(MAKE) -f ../rust/Makefile build_release

# build the Portals application in Debug mode
build: build_ockam
	# the xcode project expects the ockam executable to be found in the release directory
	mkdir -p $(ROOT_DIR)/target/release
	cp $(ROOT_DIR)/target/debug/ockam $(ROOT_DIR)/target/release
	make xcode_build_Debug

# build the Portals application in Release mode
build_release: build_ockam_release
	make xcode_build_Release

# build the Portals application in Release mode and sign it
build_signed: build_ockam_release
	make xcode_build_signed

# build the Swift code in Debug or Release mode
xcode_build_%:
	xcodebuild archive -project ockam/ockam_app/Ockam.xcodeproj/ -scheme "Portals, by Ockam" -configuration $* -archivePath build/Ockam.xcarchive
	@echo "Build complete.  See build/Ockam.xcarchive for the built app."
	@echo "To execute run: build/Ockam.xcarchive/Products/Applications/Portals,\ by\ Ockam.app/Contents/MacOS/Portals,\ by\ Ockam"

# build the Swift code in Release mode and sign the application
xcode_build_signed:
	xcodebuild archive -project ockam/ockam_app/Ockam.xcodeproj/ -scheme "Portals, by Ockam" -configuration Release -archivePath build/Ockam.xcarchive \
		-arch $(ARCH) \
		CODE_SIGN_IDENTITY="${CODE_SIGN_IDENTITY}" \
		PROVISIONING_PROFILE_SPECIFIER="${PROVISIONING_PROFILE_SPECIFIER}" \
		DEVELOPMENT_TEAM="${DEVELOPMENT_TEAM}"

# build the Portals application and package it as a .dmg file
package: build_release package_only

# package the Portals application as a .dmg file
package_only:
	xcodebuild -exportArchive -archivePath build/Ockam.xcarchive/ -exportPath build/ -exportOptionsPlist ockam/ockam_app/Ockam/ExportOptions.plist
	rm -f build/Ockam.dmg
	create-dmg \
			--no-internet-enable \
			--volname "Portals, by Ockam - Installer" --hide-extension "Portals, by Ockam.app" \
			--background ockam/ockam_app/packaging/installer-background.png \
			--window-size 600 400 --icon-size 128 \
			--icon "Portals, by Ockam.app" 126 185 --app-drop-link 466 185 \
			build/Ockam.dmg "build/Portals, by Ockam.app/"

# build the Portals application in Debug mode and start it
run: build
	"build/Ockam.xcarchive/Products/Applications/Portals, by Ockam.app/Contents/MacOS/Portals, by Ockam"

# build the Portals application in Release mode and start it
run_release: build_release
	"build/Ockam.xcarchive/Products/Applications/Portals, by Ockam.app/Contents/MacOS/Portals, by Ockam"

test:
	@echo "No test command specified."
lint:
	@echo "No lint command specified."

.PHONY: \
	clean very_clean \
	build build_release build_signed \
	package package_only \
	run run_release \
	test lint \
