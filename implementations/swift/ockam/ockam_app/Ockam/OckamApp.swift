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
        groups: [],
        sent_invitations: []
    )

    func update(state: ApplicationState) {
        debugPrint("update: ", state)
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

@main
struct OckamApp: App {
    @State var state: ApplicationState = StateContainer.shared.state
    // when the application initialization fails to load we enter a broken state
    // where we only propose a reset to the user
    var broken: Bool = false
    @Environment(\.openWindow) var openWindow

    var body: some Scene {
        MenuBarExtra {
            if broken {
                // we need to give the user a way to re-open the window
                // to provide at least a way to quit the application
                ClickableMenuEntry(text: "Open Window", action: {
                    openWindow(id: "broken-state")
                })
                .frame(width: 120,height: 40)
            } else {
                MainView(state: $state)
                    .onAppear(perform: {
                        StateContainer.shared.callback(callback: { state in
                            self.state = state
                        })
                    })
            }
        } label: {
            Image("MenuBarIcon")
                .renderingMode(.template)
                .contentShape(Rectangle())
                .buttonStyle(PlainButtonStyle())
                .onAppear(perform: {
                    if broken {
                        openWindow(id: "broken-state")
                    }
                })
        }
        .menuBarExtraStyle(.window)
        .commandsRemoved()

        Window("Could not load local state", id: "broken-state") {
            BrokenStateView()
        }
        .windowResizability(.contentSize)

        // Declare a window with an empty view to handle the ockam:// url
        // A hack to overcome the fact that `onOpenURL` only works on `Windows`
        Window("Accepting invitation", id: "accepting-invitation") {
            OpenUrlView(enrolled: $state.enrolled)
        }
        .windowResizability(.contentSize)
        // Declare a state-independent window, not open by default
        Window("Create an outlet to a tcp service", id: "create-service") {
            CreateServiceView()
        }
        .windowResizability(.contentSize)

        // Declare a "template" of windows, dependent on the LocalService.ID, not open by default
        WindowGroup("Share a service", id: "share-service", for: LocalService.ID.self) {
            $localServiceId in
            ShareServiceView(
                localService: StateContainer.shared.state.getLocalService(
                    localServiceId.unsafelyUnwrapped
                ).unsafelyUnwrapped)
        }
        .windowResizability(.contentSize)
    }

    init() {
        if !swift_initialize_application() {
            broken = true
            print("Could not initialize application: entering broken state")
        }
    }
}
