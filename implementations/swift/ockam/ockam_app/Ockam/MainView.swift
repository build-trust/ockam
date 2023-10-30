/*
    This file contain the main view when you click on the menu extra (aka system tray)
*/

import SwiftUI

struct MainView: View {
    @Environment(\.presentationMode) var presentationMode: Binding<PresentationMode>
    @Binding var state: ApplicationState
    @Environment(\.openWindow) var openWindow
    @State private var selectedGroup: String = ""
    @State private var optionPressed: Bool = false
    var timer = Timer.publish(every: 0.5, on: .main, in: .common).autoconnect()

    var body: some View {
        VStack(alignment: .leading) {
            HStack {
                VStack(alignment: .leading) {
                    Text("Ockam").font(.headline)
                    switch state.orchestrator_status {
                    case .Disconnected:
                        Text("Disconnected from the Orchestrator").font(.subheadline)
                    case .Connected:
                        Text("Connected to Orchestrator").font(.subheadline)
                    case .Connecting:
                        Text("Connecting to Orchestrator").font(.subheadline)
                    case .WaitingForToken:
                        Text("Waiting for token").font(.subheadline)
                    case .RetrievingSpace:
                        Text("Retrieving space").font(.subheadline)
                    case .RetrievingProject:
                        Text("Retrieving project").font(.subheadline)
                        Text("This might take a few minutes...").font(.caption)
                    }
                }
            }.padding(5)

            if state.enrolled {
                Group {
                    Divider()
                    HStack {
                        ProfilePicture(url: state.enrollmentImage)
                        VStack(alignment: .leading) {
                            if let name = state.enrollmentName {
                                Text(verbatim: name).font(.title3).lineLimit(1)
                            }
                            let email = state.enrollmentEmail.unsafelyUnwrapped
                            Text(verbatim: "Email: \(email)").foregroundColor(.primary.opacity(0.7))
                        }
                        Spacer()
                    }
                }
            } else {
                if state.orchestrator_status == OrchestratorStatus.Disconnected {
                    ClickableMenuEntry(
                        text: "Enroll", icon: "arrow.right.square",
                        action: {
                            enroll_user()
                            self.closeWindow()
                        })
                }
            }

            if state.enrolled {
                if !state.sent_invitations.isEmpty {
                    Group {
                        Divider()
                        SentInvitations(state: self.state)
                    }
                }

                Group {
                    Divider()
                    Text("Your services")
                        .font(.body).bold().foregroundColor(.primary.opacity(0.7))
                    ClickableMenuEntry(
                        text: "Create Service", icon: "plus",
                        action: {
                            openWindow(id: "create-service")
                            self.closeWindow()
                            bringInFront()
                        }
                    ).buttonStyle(PlainButtonStyle())
                    ForEach(state.localServices) { localService in
                        LocalServiceView(localService: localService)
                    }
                }

                if !state.groups.isEmpty {
                    Divider()
                    Text("Services shared with you")
                        .font(.body).bold().foregroundColor(.primary.opacity(0.7))

                    ForEach(state.groups) { group in
                        if selectedGroup == group.id {
                            ServiceGroupView(
                                group: group,
                                back: {
                                    selectedGroup = ""
                                }
                            )
                            .padding([.top], 5)
                        }
                    }

                    if selectedGroup == "" {
                        VStack {
                            ForEach(state.groups) { group in
                                ServiceGroupButton(
                                    group: group,
                                    action: {
                                        selectedGroup = group.id
                                    })
                            }
                        }
                        .padding([.top], 5)
                    }
                }
            }

            Group {
                Divider()
                VStack(spacing: 0) {
                    @Environment(\.openWindow) var openWindow
                    if self.optionPressed {
                        ClickableMenuEntry(
                            text: "Reset", icon: "arrow.counterclockwise",
                            action: {
                                reset_application_state()
                            })
                    }
                    ClickableMenuEntry(
                        text: "Documentation", icon: "book",
                        action: {
                            if let url = URL(string: "https://docs.ockam.io") {
                                NSWorkspace.shared.open(url)
                            }
                            self.closeWindow()
                        })
                    ClickableMenuEntry(
                        text: "Quit", icon: "power",
                        action: {
                            //even if the graceful shutdown takes a few seconds
                            //we can give a "acknowledged" feedback to the user
                            //by closing the window first
                            self.closeWindow()
                            shutdown_application()
                        })
                }
            }
        }
        .padding(6)
        .frame(width: 300)
        .onReceive(timer) { time in
            optionPressed = NSEvent.modifierFlags.contains(.option)
        }
    }

    func closeWindow() {
        self.presentationMode.wrappedValue.dismiss()
    }
}

struct MainView_Previews: PreviewProvider {
    @State static var state = swift_demo_application_state()

    static var previews: some View {
        MainView(state: $state)
    }
}
