/*
    This file is the entrypoint for the application
*/

import SwiftUI

/*
    This class was needed to allow receiving events from the C library.
    The idea is to update the state inside a static instance and
    use a callback to propagate the change back into swift-ui.

    see swift_initialize_application() in Bridge.swift
*/
class StateContainer {
    static var shared = StateContainer()

    var state = ApplicationState(
        enrolled: false,
        orchestrator_status: OrchestratorStatus.Disconnected,
        enrollmentName: nil,
        enrollmentEmail: nil,
        enrollmentImage: nil,
        enrollmentGithubUser: nil,
        localServices: [],
        groups: []
    )

    func update(state: ApplicationState) {
        print("update: \(state)")
        self.state = state
        if let callback = self.callback {
            callback(state)
        }
    }

    var callback: ((ApplicationState) -> Void)?
    func callback(callback: @escaping (ApplicationState) -> Void) {
        self.callback = callback
        callback(state)
    }
}

// Helper function to move the application windows to the front
func bringInFront() {
    NSApplication.shared.activate(ignoringOtherApps: true)
}

// Helper function to copy the text into the clipboard
func copyToClipboard(_ text: String) {
    let pasteboard = NSPasteboard.general
    pasteboard.declareTypes([.string], owner: nil)
    pasteboard.setString(text, forType: .string)
}

@main
struct OckamApp: App {
    @State var state: ApplicationState = StateContainer.shared.state;

    var body: some Scene {
        MenuBarExtra
        {
            MainView(state: $state)
                .onAppear(perform: {
                    StateContainer.shared.callback(callback: { state in
                        self.state = state
                    })
                })
                .onOpenURL(perform: { url in
                    // invoked when opening a ockam:// url
                    let urlComponents = URLComponents(url: url, resolvingAgainstBaseURL: false)
                    if let path = urlComponents?.path {
                        let segments = path.split(separator: "/", omittingEmptySubsequences: true).map(String.init)
                        if segments.count >= 2 {
                            if segments[0] == "invitations" && segments[1] == "accept" {
                                accept_invitation(segments[2])
                                return
                            }
                        }
                        print("Ignoring URL \(url)")
                    }
                })
        } label: {
            Image("MenuBarIcon")
                .renderingMode(.template)
        }
        .menuBarExtraStyle(.window)
        .commandsRemoved()

        // Declare a state-independent window, not open by default
        Window("Create a service", id: "create-service") {
            CreateServiceView()
        }
        .commandsRemoved()
        .windowResizability(.contentSize)

        // Declare a "template" of windows, dependent on the LocalService.ID, not open by default
        WindowGroup("Share a service", id: "share-service", for: LocalService.ID.self) { $localServiceId in
            ShareServiceView(localService: StateContainer.shared.state.getLocalService(
                localServiceId.unsafelyUnwrapped
            ).unsafelyUnwrapped)
        }
        .windowResizability(.contentSize)
        .commandsRemoved()
    }

    init() {
        swift_initialize_application()
    }
}

