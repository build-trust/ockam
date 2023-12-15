build: ockam_app_lib
	xcodebuild archive -project ockam/ockam_app/Ockam.xcodeproj/ -scheme "Portals, by Ockam" -configuration Debug -archivePath build/Ockam.xcarchive
	@echo "Build complete.  See build/Ockam.xcarchive for the built app."
	@echo "To execute run: build/Ockam.xcarchive/Products/Applications/Ockam.app/Contents/MacOS/Ockam"
build_release: ockam_app_lib_release xcode_build
xcode_build:
	xcodebuild archive -project ockam/ockam_app/Ockam.xcodeproj/ -scheme "Portals, by Ockam" -configuration Release -archivePath build/Ockam.xcarchive
	@echo "Build complete.  See build/Ockam.xcarchive for the built app."
	@echo "To execute run: implementations/swift/build/Ockam.xcarchive/Products/Applications/Ockam.app/Contents/MacOS/Ockam"
xcode_signed_build:
	xcodebuild archive -project ockam/ockam_app/Ockam.xcodeproj/ -scheme "Portals, by Ockam" -configuration Release -archivePath build/Ockam.xcarchive -arch $(ARCH)  CODE_SIGN_IDENTITY="${CODE_SIGN_IDENTITY}"  PROVISIONING_PROFILE_SPECIFIER="${PROVISIONING_PROFILE_SPECIFIER}" DEVELOPMENT_TEAM="${DEVELOPMENT_TEAM}"
ockam_app_lib_release:
	$(MAKE) -f ../rust/Makefile build_release_ockam_app_lib
ockam_command_release:
	$(MAKE) -f ../rust/Makefile build_release_ockam_command
ockam_app_lib:
	$(MAKE) -f ../rust/Makefile build_ockam_app_lib
package: ockam_command_release build_release package_only
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
test:
	@echo "No test command specified."
lint:
	@echo "No lint command specified."
clean:
	rm -rf "build/"
very_clean: clean
	$(MAKE) -f ../rust/Makefile clean_ockam_app_lib

.PHONY: \
	build_release build_package \
	test lint clean very_clean \
	ockam_app_lib_release ockam_app_lib \
	ockam_command_release
