import SwiftUI


struct AcceptingInvitationWrapper: View {
    @Environment(\.openWindow) private var openWindow
    
    @Binding var state: ApplicationState
    @Binding var invitationIdContainer: InvitationContainer
    
    @State private var enrollClickedFromHere: Bool = false
    @State private var showWindowOnEnrollment: Bool = true
    
    var body: some View {
        if state.orchestrator_status != .Connecting &&
            state.orchestrator_status != .Connected {
            EnrollmentBlock(
                appState: $state,
                onEnroll: {
                    enrollClickedFromHere = true
                }
            )
            .frame(height: 340)
            .padding(WindowBorderSize)
            .padding(.vertical, VerticalSpacingUnit)
            .onReceive(state.$orchestrator_status, perform: { newValue in
                if enrollClickedFromHere {
                    if newValue != .WaitingForToken && newValue != .Disconnected {
                        if showWindowOnEnrollment {
                            openWindow(id: "accepting-invitation")
                            bringInFront()
                            // only works once
                            showWindowOnEnrollment = false
                        }
                    }
                }
            })
        } else {
            AcceptingInvitation(
                state: $state,
                invitationIdContainer: $invitationIdContainer
            )
            .frame(width: 400, height: 300)
        }
    }
}

struct AcceptingInvitation: View {
    @Environment(\.presentationMode) var presentationMode: Binding<PresentationMode>
    @Environment(\.openWindow) var openWindow
    
    @Binding var state: ApplicationState
    @Binding var invitationIdContainer: InvitationContainer
    
    @State private var invitation: Optional<(ServiceGroup,Invitation)> = nil
    @State private var service: Optional<(ServiceGroup,Service)> = nil
    
    var body: some View {
        VStack(alignment: .center) {
            if !state.loaded {
                Spacer()
                Text("Loading invitations").font(.headline)
                Spacer()
                
                HStack {
                    Spacer()
                    Button(
                        action: {
                            self.closeWindow()
                        },
                        label: {
                            Text("Dismiss")
                        }
                    )
                    .keyboardShortcut(.defaultAction)
                }
                .padding(.vertical, VerticalSpacingUnit)
                .padding(.horizontal, HorizontalSpacingUnit)
                .background(OckamDarkerBackground)
            } else if let (group, invitation) = self.invitation {
                Spacer()
                
                HStack {
                    Spacer()
                    ProfilePicture(url: group.imageUrl, size: 64)
                    VStack(alignment: .leading) {
                        if let name = group.name {
                            Text(verbatim: name)
                        }
                        Text(verbatim: group.email)
                    }
                    Spacer()
                }
                .padding(.vertical, VerticalSpacingUnit * 2)
                
                Group {
                    Text("Has invited you to the portal:")
                        .padding(.vertical, VerticalSpacingUnit)
                        .padding(.horizontal, HorizontalSpacingUnit)
                        .font(.headline)
                    Text(invitation.serviceName)
                    if let scheme = invitation.serviceScheme {
                        Text(verbatim: scheme).font(.caption)
                    }
                }.padding(0)
                
                Spacer()
                    .frame(height: VerticalSpacingUnit)
                Spacer()
                
                HStack {
                    Spacer()
                    Button(
                        action: {
                            accept_invitation(invitationIdContainer.id)
                            self.closeWindow()
                        },
                        label: {
                            Text("Accept")
                        }
                    )
                    if state.enrolled {
                        Button(
                            action: {
                                ignore_invitation(invitationIdContainer.id)
                                self.closeWindow()
                            },
                            label: {
                                Text("Decline")
                            }
                        )
                    }
                    Button(
                        action: {
                            self.closeWindow()
                        },
                        label: {
                            Text("Dismiss")
                        }
                    )
                    .keyboardShortcut(.defaultAction)
                }
                .padding(.vertical, VerticalSpacingUnit)
                .padding(.horizontal, HorizontalSpacingUnit)
                .background(OckamDarkerBackground)
            } else if let (group, service) = self.service {
                Spacer()
                
                HStack {
                    Spacer()
                    ProfilePicture(url: group.imageUrl, size: 64)
                    VStack(alignment: .leading) {
                        if let name = group.name {
                            Text(verbatim: name)
                        }
                        Text(verbatim: group.email)
                    }
                    Spacer()
                }
                .padding(.horizontal, VerticalSpacingUnit * 2)
                .padding(.vertical, HorizontalSpacingUnit * 2)
                
                Text("This invitation has already been accepted.")
                    .padding(.vertical, VerticalSpacingUnit)
                    .padding(.horizontal, HorizontalSpacingUnit)
                    .font(.headline)
                
                Group {
                    Text(service.sourceName)
                    if let scheme = service.scheme {
                        Text(verbatim: scheme).font(.caption)
                    }
                }.padding(0)
                
                Spacer()
                    .frame(height: 10)
                Spacer()
                
                HStack {
                    Spacer()
                    Button(
                        action: {
                            self.closeWindow()
                        },
                        label: {
                            Text("Dismiss")
                        }
                    )
                    .keyboardShortcut(.defaultAction)
                }
                .padding(.vertical, VerticalSpacingUnit)
                .padding(.horizontal, HorizontalSpacingUnit)
                .background(OckamDarkerBackground)
            } else {
                Spacer()
                Text("This invitation cannot be accepted.")
                    .padding(.top, VerticalSpacingUnit)
                    .padding(.bottom, 0)
                    .font(.headline)
                Text("This invitation has either expired, was revoked, or was intended for a different account.\nPlease contact the sender of the invitation for more information.")
                    .padding(10)
                Spacer()
                
                HStack {
                    Spacer()
                    Button(
                        action: {
                            self.closeWindow()
                        },
                        label: {
                            Text("Dismiss")
                        }
                    )
                    .keyboardShortcut(.defaultAction)
                }
                .padding(.vertical, VerticalSpacingUnit)
                .padding(.horizontal, HorizontalSpacingUnit)
                .background(OckamDarkerBackground)
            }
        }
        .onReceive(state.$groups) { _ in
            refreshStateFromInvitationId(self.invitationIdContainer.id)
        }
        .onReceive(invitationIdContainer.$id, perform: { invitationId in
            refreshStateFromInvitationId(invitationId)
        })
    }
    
    func closeWindow() {
        presentationMode.wrappedValue.dismiss()
    }
    
    func refreshStateFromInvitationId(_ invitationId: String) {
        logger.debug("Refreshing invitation from \(invitationId)")
        if invitationId == "" {
            invitation = nil
            service = nil
        } else {
            invitation = state.lookupInvitationById(invitationId)
            service = state.lookupIncomingServiceById(invitationId)
        }
    }
}

struct OpenUrlView_Previews: PreviewProvider {
    @State static var state = swift_demo_application_state()
    @State static var invitationIdContainer = InvitationContainer()
    
    static var previews: some View {
        AcceptingInvitationWrapper(
            state: $state,
            invitationIdContainer: $invitationIdContainer
        )
        .onAppear(perform: {
            Self.invitationIdContainer.id = "5373"
        })
    }
}
