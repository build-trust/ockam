build: ockam_app_lib
	xcodebuild archive -project ockam/ockam_app/Ockam.xcodeproj/ -scheme Ockam -configuration Debug -archivePath build/Ockam.xcarchive
	@echo "Build complete.  See build/Ockam.xcarchive for the built app."
	@echo "To execute run: build/Ockam.xcarchive/Products/Applications/Ockam.app/Contents/MacOS/Ockam"
build_release: ockam_app_lib_release
	xcodebuild archive -project ockam/ockam_app/Ockam.xcodeproj/ -scheme Ockam -configuration Release -archivePath build/Ockam.xcarchive
	@echo "Build complete.  See build/Ockam.xcarchive for the built app."
	@echo "To execute run: build/Ockam.xcarchive/Products/Applications/Ockam.app/Contents/MacOS/Ockam"
ockam_app_lib_release:
	$(MAKE) -f ../rust/Makefile build_release_ockam_app_lib
ockam_command_release:
	$(MAKE) -f ../rust/Makefile build_release_ockam_command
ockam_app_lib:
	$(MAKE) -f ../rust/Makefile build_ockam_app_lib
package: build_release ockam_command_release
	xcodebuild -exportArchive -archivePath build/Ockam.xcarchive/ -exportPath build/ -exportOptionsPlist ockam/ockam_app/Ockam/ExportOptions.plist
	cp ../../target/release/ockam build/Ockam.app/Contents/MacOS/ockam_command
	rm -f build/Ockam.dmg
	create-dmg \
			--no-internet-enable \
			--volname "Ockam Installer" --hide-extension Ockam.app \
			--background ockam/ockam_app/packaging/installer-background.png \
			--window-size 600 400 --icon-size 128 \
			--icon Ockam.app 126 185 --app-drop-link 466 185 \
			build/Ockam.dmg build/Ockam.app
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
