//
// This file is the entrypoint for the application
//

import SwiftUI
import AppKit
import OSLog
import UserNotifications


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
    private var urlOpened = false

    func applicationDidFinishLaunching(_ notification: Foundation.Notification) {
        // avoid creating the menubar extra for preview purposes
        if isRunningPreviewMode() {
            return
        }

        // we don't want any swiftui window to be automatically open at startup
        // and the first window is "accepting-invitation"
        for window in NSApplication.shared.windows {
            if let id = window.identifier {
                if id.rawValue == "accepting-invitation" {
                    window.close()
                }
            }
        }

        // this instance is responsible to handle the notifications
        setupNotifications()

        self.menuBarExtra = FluidMenuBarExtra(title: "Ockam", image: "MenuBarIcon") {
            // create the view and expose the AppDelegate instance to allow
            // the user to close the popover at will
            WrapperView().environmentObject(self)
        }

        // shows the main window at bootstrap, the idea is that new users may not notice
        // the menu extra icon and by opening it by default it becomes evident, and
        // when opening the application the user is likely to interact with it right away.

        // the position of the popover is dependent on the menuextra status position
        // which is not yet defined, we need to wait a bit to correctly position the
        // popover
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.2) {
            // we don't want to hide the acceptance window when we start
            // the application from a link
            if !self.urlOpened {
                self.showPopover()
            }
        }
    }

    // avoid opening the default window when clickin on notifications
    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
        return false
    }

    // avoid closing the application when closing the last window
    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        return false
    }

    // we don't want the OS to keep track of which windows were open, it only causes
    // extra confusion
    func applicationSupportsSecureRestorableState(_ application: NSApplication) -> Bool {
        return false
    }

    func application(_ application: NSApplication, open urls: [URL]) {
        for url in urls {
            logger.info("Received url: \(url.absoluteString, privacy: .public)")
            if let invitationId = parseInvitationIdFromUrl(url: url) {
                urlOpened = true
                InvitationContainer.shared.update(invitationId: invitationId)
            }
        }
    }

    func setupNotifications() {
        let center = UNUserNotificationCenter.current()
        center.delegate = self
        center.requestAuthorization(options: [.alert, .sound, .badge]) { (granted, error) in
            if granted {
                print("Notification permission granted.")
            } else {
                print("Notification permission denied.")
            }
        }
    }

    func showPopover() {
        self.menuBarExtra?.showWindow()
    }

    func dismissPopover() {
        // needed an explicit dismissal since the default @Environmet(\.dismiss\)
        // is not working properly for the popover window
        self.menuBarExtra?.dismissWindow()
    }
}

extension AppDelegate: UNUserNotificationCenterDelegate {
    func userNotificationCenter(_ center: UNUserNotificationCenter, willPresent notification: UNNotification, withCompletionHandler completionHandler: @escaping (UNNotificationPresentationOptions) -> Void) {
        completionHandler([.sound, .badge])
    }

    func userNotificationCenter(_ center: UNUserNotificationCenter, didReceive response: UNNotificationResponse, withCompletionHandler completionHandler: @escaping () -> Void) {
        // upon receiving a notification show the popover window
        showPopover()
        completionHandler()
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

// On macOS ventura openWindow() doesn't work since the SwiftUI
// lifecycle is not recognized for some reason when using NSHostingView,
// this has been fixed in sonoma.
// The idea is to trigger an event to trigger an openWindow from another
// context where NSHostingView is a parent view.
// Only openWindow() used within the PopOver is affected.
class OpenWindowWorkaround: ObservableObject {
    static var shared = OpenWindowWorkaround()
    @Published var windowName = ""
    @Published var value = ""

    func openWindow(windowName: String) {
        self.windowName = windowName;
        self.value = "";
    }

    func openWindow(windowName: String, value: String) {
        self.windowName = windowName;
        self.value = value;
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
            AcceptingInvitationWrapper(state: $state, invitationIdContainer: $invitation)
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
                        logger.info("opening 'accepting-invitation' window with invitation \(invitationId, privacy: .public)")
                        // without this it won't show the window when a link is clicked and the application
                        // has not yet started
                        openWindow(id: "accepting-invitation")
                    }
                })
            // hack for ventura, see OpenWindowWorkaround comment
                .onReceive(OpenWindowWorkaround.shared.$windowName) { _ in
                    if OpenWindowWorkaround.shared.value == "" {
                        openWindow(
                            id: OpenWindowWorkaround.shared.windowName
                        )
                    } else {
                        openWindow(
                            id: OpenWindowWorkaround.shared.windowName,
                            value: OpenWindowWorkaround.shared.value
                        )
                    }
                }
        }
        .windowResizability(.contentSize)

        WindowGroup("Confirmation", id: "delete-portal-confirmation", for: Service.ID.self) { $serviceId in
            if let serviceId = serviceId {
                DeleteIncomingPortalView(
                    service: StateContainer.shared.state.lookupIncomingServiceById(
                        serviceId
                    ).unsafelyUnwrapped.1
                )
            }
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
        Window("Open a Portal Outlet to a TCP service", id: "open-portal") {
            OpenPortal(localServices: $state.localServices)
        }
        .windowResizability(.contentSize)

        // Declare a "template" of windows, dependent on the LocalService.ID, not open by default
        WindowGroup("Invite your friends to this Portal", id: "invite-to-portal", for: LocalService.ID.self) {
            $localServiceId in
            InviteToPortal(
                state_loaded: $state.loaded,
                localService: StateContainer.shared.state.getLocalService(
                    localServiceId.unsafelyUnwrapped
                ).unsafelyUnwrapped)
        }
        .windowResizability(.contentSize)
    }

    init() {
        // avoid initialization when previewing
        if isRunningPreviewMode() {
            return
        }

        logger.info("Application started")
        if swift_initialize_application() {
            logger.info("Application successfully initialized")
        } else {
            broken = true
            logger.error("Could not initialize application: entering broken state")
        }
    }
}
