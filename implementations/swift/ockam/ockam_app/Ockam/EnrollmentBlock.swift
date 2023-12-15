
import SwiftUI


struct EnrollmentBlock: View {
    @Environment(\.colorScheme) var colorScheme

    @Binding var status: OrchestratorStatus

    @State var invitation: InvitationContainer = InvitationContainer.shared
    @State var onEnroll: (() -> Void)? = nil

    var body: some View {
        VStack(spacing: 0) {
            VStack(spacing: 0) {
                if status == OrchestratorStatus.Disconnected {
                    Text("Portals, by Ockam")
                        .bold()
                        .padding(.bottom, VerticalSpacingUnit*2)

                    Text(
"""
Privately share a TCP or HTTP service from this Mac to anyone, anywhere. It is shared securely over an end-to-end encrypted Ockam Portal.

Your friends will have access to it on their **localhost**!
"""
                    )

                    EnrollmentStatus(status: $status)
                        .padding(.vertical, VerticalSpacingUnit*2)

                    Button(action: {
                        enroll_user()
                        if let onEnroll = onEnroll {
                            onEnroll()
                        }
                    }) {
                        Text("Enroll…")
                            .frame(
                                width: HorizontalSpacingUnit*10,
                                height: VerticalSpacingUnit*3
                            )
                    }
                    .controlSize(.large)
                    .keyboardShortcut(.defaultAction)

                } else if status != OrchestratorStatus.Connecting &&
                            status != OrchestratorStatus.Connected {
                    Text("Portals, by Ockam")
                        .bold()
                        .padding(.bottom, VerticalSpacingUnit*2)


                    if status == OrchestratorStatus.WaitingForToken {
                        Image("EnrollmentPage")
                            .resizable()
                            .aspectRatio(contentMode: .fit)
                            .frame(height: 150)
                    }

                    if status == OrchestratorStatus.RetrievingSpace ||
                         status == OrchestratorStatus.RetrievingProject {
                        RotatingText(
                            texts: [
                                "Ockam Orchestrator runs Encrypted Cloud Relays for your services so that they can be accessible from anywhere over end-to-end encrypted Portals.",
                                "Portals can traverse NATs, firewalls, and clouds without any change to networks or infrastructure.",
                                "Portals are always mutually authenticated. Your data is only available to you and your invited friends. No one, not even Ockam Orchestrator, can see or tamper with your data."
                            ],
                            interval: 15.0
                        )
                    }

                    EnrollmentStatus(status: $status)
                        .padding(.vertical, VerticalSpacingUnit*2)

                    if status == OrchestratorStatus.WaitingForToken {
                        Button(action: {
                            restartCurrentProcess()
                        }) {
                            Text("Start Enrollment Again…")
                                .frame(
                                    width: HorizontalSpacingUnit*20,
                                    height: VerticalSpacingUnit*3
                                )
                        }
                        .controlSize(.large)
                    }
                } else if status == OrchestratorStatus.Connecting ||
                            status == OrchestratorStatus.Connected {
                    Text("Portals, by Ockam")
                        .bold()
                        .padding(.bottom, VerticalSpacingUnit*2)


                    Text(
"""
You are now enrolled with Ockam Orchestrator. We've set up an encrypted relay for you.

First, open a new Portal Outlet to a service that is accessible from your computer. Then, invite your friends to it.
"""
                    )
                }
            }
            .frame(maxWidth: .infinity)
            .padding(VerticalSpacingUnit*2)
            .background( colorScheme == .dark ?
                Color.black.opacity(0.1) :
                Color.white.opacity(0.2)
            )
            .overlay(
                RoundedRectangle(cornerRadius: 4)
                    .stroke( colorScheme == .dark ?
                        AnyShapeStyle(Color.white.opacity(0.2)) :
                        AnyShapeStyle(Color.black.opacity(0.1)),
                    lineWidth: 1
                )
            )
            .cornerRadius(4)
        }
        .padding(.horizontal, WindowBorderSize + HorizontalSpacingUnit)
    }
}


struct EnrollmentBlock_Previews: PreviewProvider {
    static var previews: some View {
        EnrollmentBlock(
            status: .constant(.Connected)
        )
        .frame(width: 300, height: 440)
    }
}
