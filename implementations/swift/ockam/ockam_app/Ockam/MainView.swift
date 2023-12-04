import SwiftUI

struct MainView: View {
    @Environment(\.openWindow) private var openWindow
    @EnvironmentObject private var appDelegate: AppDelegate

    @Binding var state: ApplicationState
    @State private var selectedGroup: String = ""
    @State private var optionPressed: Bool = false

    var timer = Timer.publish(every: 0.5, on: .main, in: .common).autoconnect()

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            if state.enrolled {
                Group {
                    HStack(spacing: HorizontalSpacingUnit) {
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
                .padding(.top, VerticalSpacingUnit)
                .padding(.horizontal, HorizontalSpacingUnit)
            } else {
                EnrollmentStatus(
                    status: $state.orchestrator_status
                )
                .padding(.horizontal, HorizontalSpacingUnit)

                if state.orchestrator_status == OrchestratorStatus.Disconnected {
                    ClickableMenuEntry(
                        text: "Enroll...", icon: "arrow.right.square",
                        action: {
                            enroll_user()
                            appDelegate.dismissPopover()
                        })
                    .padding(.top, VerticalSpacingUnit)
                    .padding(.horizontal, HorizontalSpacingUnit)
                }
            }

            if state.enrolled {
                if selectedGroup == "" {
                    Group {
                        Text("Your services:")
                            .font(.body).bold()
                            .foregroundColor(OckamSecondaryTextColor)
                            .padding(.top, VerticalSpacingUnit*2)
                            .padding(.bottom, VerticalSpacingUnit)

                        ForEach(state.localServices) { localService in
                            LocalServiceView(localService: localService)
                        }

                        ClickableMenuEntry(
                            text: "Create a service...",
                            action: {
                                openWindow(id: "create-service")
                                bringInFront()
                            }
                        )
                        .buttonStyle(PlainButtonStyle())
                        .padding(.top, VerticalSpacingUnit)
                    }
                }

                if !state.groups.isEmpty {
                    Text("Services shared with you:")
                        .font(.body).bold()
                        .foregroundColor(OckamSecondaryTextColor)
                        .padding(.top, VerticalSpacingUnit*2)
                        .padding(.bottom, VerticalSpacingUnit)

                    ForEach(state.groups) { group in
                        if selectedGroup == "" || selectedGroup == group.email {
                            ServiceGroupView(
                                group: group,
                                back: {
                                    selectedGroup = ""
                                },
                                action: {
                                    selectedGroup = group.email
                                }
                            )
                        }
                    }
                }
            }

            if selectedGroup == "" {
                if !state.sent_invitations.isEmpty {
                    SentInvitations(state: self.state)
                        .padding(.top, VerticalSpacingUnit)
                }

                Group {
                    Divider()
                        .padding(.vertical, VerticalSpacingUnit)
                    VStack(spacing: 0) {
                        @Environment(\.openWindow) var openWindow
                        ClickableMenuEntry(
                            text: "Star us on Github...", icon: "star",
                            action: {
                                if let url = URL(string: "https://github.com/build-trust/ockam") {
                                    NSWorkspace.shared.open(url)
                                }
                                appDelegate.dismissPopover()
                            })
                        ClickableMenuEntry(
                            text: "Learn more from our documentation...", icon: "book",
                            action: {
                                if let url = URL(string: "https://docs.ockam.io") {
                                    NSWorkspace.shared.open(url)
                                }
                                appDelegate.dismissPopover()
                            })
                    }
                }

                Group {
                    Divider()
                        .padding(.vertical, VerticalSpacingUnit)
                    VStack(spacing: 0) {
                        if self.optionPressed {
                            ClickableMenuEntry(
                                text: "About", icon: "questionmark.circle",
                                action: {
                                    openWindow(id: "about")
                                    bringInFront()
                                })
                            ClickableMenuEntry(
                                text: "Reset", icon: "arrow.counterclockwise",
                                action: {
                                    restartCurrentProcess()
                                })
                            Divider()
                                .padding([.top,.bottom], VerticalSpacingUnit)
                        }
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
            }
        }
        .padding(.vertical, VerticalSpacingUnit)
        .padding(.horizontal, HorizontalSpacingUnit)
        .frame(width: 320)
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
    }
}
