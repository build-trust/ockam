//
// This file is the entrypoint for the application
//

import SwiftUI
import AppKit
import OSLog
import FluidMenuBarExtra


// you can read the logs inside the console application
let logger: Logger = Logger(
    subsystem: Bundle.main.bundleIdentifier!,
    category: String(describing: OckamApp.self)
)

// This class was needed to allow receiving events from the C library.
// The idea is to update the state inside a static instance and
// use a callback to propagate the change back into swift-ui.
//
// see swift_initialize_application() in Bridge.swift
class StateContainer {
    static var shared = StateContainer()

    var callbacks: [((ApplicationState) -> Void)] = []
    var state = ApplicationState(
        enrolled: false,
        loaded: false,
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
        debugPrint(state)
        self.state = state
        for callback in self.callbacks {
            callback(state)
        }
    }

    func callback(_ callback: @escaping (ApplicationState) -> Void) {
        self.callbacks.append(callback)
        callback(state)
    }
}

struct WrapperView: View {
    @Environment(\.openWindow) var openWindow
    @State private var state = StateContainer.shared.state
    @State public var brokenState = broken

    var body: some View {
        if brokenState {
            // we need to give the user a way to re-open the window
            // to provide at least a way to quit the application
            ClickableMenuEntry(text: "Open Window", action: {
                openWindow(id: "broken-state")
            })
            .frame(width: 120,height: 40)
        } else {
            MainView(state: $state)
                .onAppear {
                    StateContainer.shared.callback{ state in
                        self.state = state
                    }
                }
        }
    }
}

// This is needed to properly receive and handle every url event, swiftui
// does have a simpler mechanism, however it doesn't handle the first time
// a link is clicked and the application is not yet started
class AppDelegate: NSObject, NSApplicationDelegate, ObservableObject {
    private var menuBarExtra: FluidMenuBarExtra?

    func applicationDidFinishLaunching(_ notification: Foundation.Notification) {
        self.menuBarExtra = FluidMenuBarExtra(title: "My Menu", image: "MenuBarIcon") {
            WrapperView()
        }
        // we don't want any window to be automatically open at startup
        if let window = NSApplication.shared.windows.first {
            window.close()
        }
    }

    func application(_ application: NSApplication, open urls: [URL]) {
        for url in urls {
            logger.info("Received url: \(url.absoluteString)")
            if let invitationId = parseInvitationIdFromUrl(url: url) {
                InvitationContainer.shared.update(invitationId: invitationId)
            }
        }
    }
}

// This is needed to create a bridge between a static context and the SwiftUI world
class InvitationContainer: ObservableObject {
    static var shared = InvitationContainer()
    @Published var id = ""

    func update(invitationId: String) {
        self.id = invitationId;
    }
}

// when the application initialization fails to load we enter a broken state
// where we only propose a reset to the user
var broken = false

@main
struct OckamApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) var appDelegate

    @Environment(\.openWindow) var openWindow

    @State var state: ApplicationState = StateContainer.shared.state
    @State var invitation: InvitationContainer = InvitationContainer.shared

    var body: some Scene {
        Window("Accepting invitation", id: "accepting-invitation") {
            AcceptingInvitation(state: $state, invitationIdContainer: $invitation)
            // no particular reason to attach .onAppear to this window, we just need a View event
            // during initialization. onAppear is meant apperance in the hierarchy and not 'visible'.
                .onAppear(perform: {
                    if broken {
                        openWindow(id: "broken-state")
                    } else {
                        StateContainer.shared.callback{ state in
                            self.state = state
                        }
                    }
                })
                .onReceive(invitation.$id, perform: { invitationId in
                    if invitationId != "" {
                        logger.info("opening 'accepting-invitation' window with invitation \(invitationId)")
                        // without this it won't show the window when a link is clicked and the application
                        // has not yet started
                        openWindow(id: "accepting-invitation")
                    }
                })
        }
        .windowResizability(.contentSize)

        WindowGroup("Confirmation", id: "decline-service-confirmation", for: Service.ID.self) { $serviceId in
            IgnoreServiceView(
                service: StateContainer.shared.state.lookupIncomingServiceById(
                    serviceId.unsafelyUnwrapped
                ).unsafelyUnwrapped.1
            )
        }
        .windowResizability(.contentSize)

        Window("About", id:"about") {
            About(runtimeInformation: swift_runtime_information())
        }
        .windowResizability(.contentSize)

        // Declare a window being shown when the ockam state cannot be loadeds
        Window("Could not load local state", id: "broken-state") {
            BrokenStateView()
        }
        .windowResizability(.contentSize)

        // Declare a state-independent window, not open by default
        Window("Create an outlet to a tcp service", id: "create-service") {
            CreateServiceView(state_loaded: $state.loaded)
        }
        .windowResizability(.contentSize)

        // Declare a "template" of windows, dependent on the LocalService.ID, not open by default
        WindowGroup("Share a service", id: "share-service", for: LocalService.ID.self) {
            $localServiceId in
            ShareServiceView(
                state_loaded: $state.loaded,
                localService: StateContainer.shared.state.getLocalService(
                    localServiceId.unsafelyUnwrapped
                ).unsafelyUnwrapped)
        }
        .windowResizability(.contentSize)
    }

    init() {
        logger.info("Application started")
        if !swift_initialize_application() {
            broken = true
            logger.error("Could not initialize application: entering broken state")
        }
    }
}
