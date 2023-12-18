import SwiftUI

struct MainView: View {
    @EnvironmentObject private var appDelegate: AppDelegate

    @Binding var state: ApplicationState
    @State private var selectedGroup: String = ""
    @State private var optionPressed: Bool = false
    @State private var enrollClickedFromHere: Bool = false
    @State private var showWindowOnEnrollment: Bool = true

    var timer = Timer.publish(every: 0.5, on: .main, in: .common).autoconnect()

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            if state.enrolled {
                Group {
                    HStack(alignment: .top, spacing: HorizontalSpacingUnit) {
                        ProfilePicture(url: state.enrollmentImage)
                        VStack(alignment: .leading) {
                            EnrollmentStatus(
                                status: $state.orchestrator_status
                            )
                            if let name = state.enrollmentName {
                                Text(verbatim: name)
                                    .lineLimit(1)
                            }
                            let email = state.enrollmentEmail.unsafelyUnwrapped
                            Text(verbatim: email)
                                .foregroundColor(OckamSecondaryTextColor)
                                .lineLimit(1)
                        }
                        Spacer()
                    }
                }
                .padding(.top, WindowBorderSize)
                .padding(.bottom, 3)
                .padding(.horizontal, WindowBorderSize + HorizontalSpacingUnit)
            }

            if state.localServices.isEmpty || (state.loaded && state.sent_invitations.isEmpty) {
                if state.enrolled {
                    Divider()
                        .padding(.top, VerticalSpacingUnit)
                        .padding(.bottom, VerticalSpacingUnit)
                        .padding(.horizontal, WindowBorderSize + HorizontalSpacingUnit)
                }

                // hide it completely once the first portal has been
                // created
                EnrollmentBlock(
                    appState: $state,
                    onEnroll: {
                        enrollClickedFromHere = true
                    }
                )
                .padding(.top, WindowBorderSize )
                .padding(.bottom, WindowBorderSize )
                .onReceive(state.$orchestrator_status, perform: { newValue in
                    if enrollClickedFromHere {
                        if newValue != .WaitingForToken && newValue != .Disconnected {
                            if showWindowOnEnrollment {
                                appDelegate.showPopover()
                                bringInFront()
                                // only works once
                                showWindowOnEnrollment = false
                            }
                        }
                    }
                })
            }

            Divider()
                .padding(.top, VerticalSpacingUnit)
                .padding(.bottom, VerticalSpacingUnit)
                .padding(.horizontal, WindowBorderSize + HorizontalSpacingUnit)

            if state.enrolled {
                ClickableMenuEntry(
                    text: "Open a new Portal Outlet…", icon: "arrow.down.backward.and.arrow.up.forward",
                    action: {
                        OpenWindowWorkaround.shared.openWindow(
                            windowName: "open-portal"
                        )
                        bringInFront()
                    }
                )
                .padding(.horizontal, WindowBorderSize)

                Divider()
                    .padding(.top, VerticalSpacingUnit)
                    .padding(.bottom, VerticalSpacingUnit)
                    .padding(.horizontal, WindowBorderSize + HorizontalSpacingUnit)

                if !state.localServices.isEmpty {
                    Text("Opened Portal Outlets")
                        .font(.subheadline).bold()
                        .foregroundColor(OckamSecondaryTextColor)
                        .padding(.bottom, 4)
                        .padding(.horizontal, WindowBorderSize + HorizontalSpacingUnit)

                    // TODO: add scrollPosition() support after ventura has been dropped
                    ScrollView {
                        VStack(spacing: 0) {
                            ForEach(state.localServices) { localService in
                                LocalPortalView(
                                    localService: localService
                                )
                            }
                        }
                    }
                    .scrollIndicators(ScrollIndicatorVisibility.never)
                    .frame(maxHeight: 350)

                    Divider()
                        .padding(.top, VerticalSpacingUnit)
                        .padding(.bottom, VerticalSpacingUnit)
                        .padding(.horizontal, WindowBorderSize + HorizontalSpacingUnit)
                }

                if !state.groups.isEmpty {
                    Text("Accessible Portal Inlets")
                        .font(.subheadline).bold()
                        .foregroundColor(OckamSecondaryTextColor)
                        .padding(.bottom, 4)
                        .padding(.horizontal, WindowBorderSize + HorizontalSpacingUnit)

                    ScrollView {
                        VStack(spacing: 0) {
                            ForEach(state.groups) { group in
                                ServiceGroupView(
                                    group: group
                                )
                            }
                        }
                    }
                    .scrollIndicators(ScrollIndicatorVisibility.never)
                    .frame(maxHeight: 350)

                    Divider()
                        .padding(.top, VerticalSpacingUnit)
                        .padding(.bottom, VerticalSpacingUnit)
                        .padding(.horizontal, WindowBorderSize + HorizontalSpacingUnit)
                }
            }

            if !state.sent_invitations.isEmpty {
                SentInvitations(state: self.state)
                Divider()
                    .padding(.vertical, VerticalSpacingUnit)
                    .padding(.horizontal, WindowBorderSize + HorizontalSpacingUnit)
            }

            Group {
                VStack(spacing: 0) {
                    @Environment(\.openWindow) var openWindow
                    ClickableMenuEntry(
                        text: "Star us on GitHub…", icon: "star",
                        action: {
                            if let url = URL(string: "https://github.com/build-trust/ockam") {
                                NSWorkspace.shared.open(url)
                            }
                            appDelegate.dismissPopover()
                        })
                    ClickableMenuEntry(
                        text: "Co-sponsor open source maintainers...", icon: "heart",
                        action: {
                            if let url = URL(string: "https://github.com/sponsors/build-trust") {
                                NSWorkspace.shared.open(url)
                            }
                            appDelegate.dismissPopover()
                        })

                    let bookIcon = if #available(macOS 14, *) {
                        "book.pages"
                    } else {
                        "book"
                    }
                    ClickableMenuEntry(
                        text: "Learn how we built this app…",
                        icon: bookIcon,
                        action: {
                            if let url = URL(string: "https://github.com/build-trust/ockam/blob/develop/examples/app/portals/README.md") {
                                NSWorkspace.shared.open(url)
                            }
                            appDelegate.dismissPopover()
                        })
                    ClickableMenuEntry(
                        text: "Share your thoughts on Discord…", icon: "message.badge",
                        action: {
                            if let url = URL(string: "https://discord.ockam.io") {
                                NSWorkspace.shared.open(url)
                            }
                            appDelegate.dismissPopover()
                        })
                }
                .padding(.horizontal, WindowBorderSize)

                Group {
                    Divider()
                        .padding(.horizontal, HorizontalSpacingUnit)
                        .padding(.vertical, VerticalSpacingUnit)

                    VStack(spacing: 0) {
                        if self.optionPressed {
                            ClickableMenuEntry(
                                text: "About...", icon: "questionmark.circle",
                                action: {
                                    OpenWindowWorkaround.shared.openWindow(
                                        windowName: "about"
                                    )
                                    bringInFront()
                                })
                            Divider()
                                .padding([.top,.bottom], VerticalSpacingUnit)
                                .padding(.horizontal, HorizontalSpacingUnit)
                        }
                        ClickableMenuEntry(
                            text: "Reset", icon: "arrow.counterclockwise",
                            action: {
                                restartCurrentProcess()
                            })
                        ClickableMenuEntry(
                            text: "Quit Ockam", icon: "power", shortcut: "⌘Q",
                            action: {
                                //even if the graceful shutdown takes a few seconds
                                //we can give a "acknowledged" feedback to the user
                                //by closing the window first
                                appDelegate.dismissPopover()
                                shutdown_application()
                            }
                        ).keyboardShortcut("Q", modifiers: .command)
                    }
                }
                .padding(.horizontal, WindowBorderSize)
            }
        }
        .padding(.vertical, WindowBorderSize)
        .frame(width: 300)
        .onReceive(timer) { time in
            optionPressed = NSEvent.modifierFlags.contains(.option)
        }
        .onReceive(state.$groups) { _ in
            // the selected group could have been deleted, if so, reset the selection
            if selectedGroup != "" {
                if !state.groups.contains(where: { $0.id == selectedGroup }) {
                    selectedGroup = ""
                }
            }
        }
    }
}

struct MainView_Previews: PreviewProvider {
    @State static var state = swift_demo_application_state()

    static var previews: some View {
        MainView(state: $state)
            .frame(height: 600)
    }
}
