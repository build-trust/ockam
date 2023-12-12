
import SwiftUI


struct GuidedIntro: View {
    @Binding var status: OrchestratorStatus
    @State private var page = 0
    @State var onFinish: (() -> Void)? = nil
    let enrollmentPage = 2
    let lastPage = 3

    var body: some View {
        Group {
            HStack {
                Spacer()
                VStack(spacing: VerticalSpacingUnit) {
                    switch(page) {
                    case 0:
                        Text("What is it?")
                            .font(.title)
                            .padding(.vertical, VerticalSpacingUnit*2)

                        Text(
"""
Ockam.app is a desktop app for macOS that makes it really easy to share private self-hosted services with your friends, without exposing these services to the Internet.
"""
                        )

                        Image("Diagram")
                            .resizable()
                            .aspectRatio(contentMode: .fit)
                            .frame(height: 300)

                        Text(
"""
Let’s say that you have a TCP service that you can access from your computer. You can share that resource with a friend so they can use it. It is as easy as creating a portal from your computer to your friend’s computer. No need to change any network settings or modify any firewall policies on your machine or theirs.
"""
                        )

                    case 1:
                        Text("How to enroll")
                            .font(.title)
                            .padding(.vertical, VerticalSpacingUnit*2)

                        Text(
"""
This is the process by which you will enroll your machine with the Ockam Orchestrator. Ockam Orchestrator is a service that lets you easily create end-to-end encrypted relays in the cloud.

When you run Enroll, the following steps take place:

    → The app creates a space for you to host your projects,
      as well as a default project for you within this space.

    → It also generates a unique cryptographically provable identity
      and saves the corresponding key in a vault. This identity is
      issued a membership credential that will be used to manage
      the resources in your project.
"""
                        )

                    case 2:
                        Text("Enroll using email & password or GitHub")
                            .font(.title)
                            .padding(.vertical, VerticalSpacingUnit*2)

                        if status == OrchestratorStatus.Disconnected {
                            Text(
"""
During the enrollment process, your web browser will be launched with a link that will ask you to authenticate.
"""                         )

                            Image("EnrollmentPage")
                                .resizable()
                                .aspectRatio(contentMode: .fit)
                                .frame(height: 300)
                        } else {
                            if status == OrchestratorStatus.Connecting ||
                                status == OrchestratorStatus.Connected {
                                Text("Your are now enrolled with Ockam Orchestrator!").font(.title3)
                            }
                            EnrollmentStatus(status: $status)
                        }

                    case 3:
                        Text("Welcome to Ockam! 🎉")
                            .font(.title)
                            .padding(.vertical, VerticalSpacingUnit*2)

                        Text(
"""
You have enrolled, what can you do now?
You can create a portal to a service and share it with your friend.
You can accept an invitation from a friend to access their portal!

For more information check out the [user guide](https://docs.ockam.io/reference/app)
"""
                        )

                    default:
                        EmptyView()
                    }

                    if page == enrollmentPage {
                        Spacer()
                        if status == OrchestratorStatus.Disconnected {
                            Button(action: {
                                enroll_user()
                            }) {
                                Text("Enroll…")
                                    .font(.title3)
                                    .frame(
                                        width: HorizontalSpacingUnit*16,
                                        height: VerticalSpacingUnit*4
                                    )
                            }
                            .controlSize(.large)
                            .keyboardShortcut(.defaultAction)
                            .padding(.vertical, VerticalSpacingUnit)
                        } else if status == OrchestratorStatus.Connecting ||
                                    status == OrchestratorStatus.Connected {
                            Button(action: {
                                page += 1
                            }) {
                                Text("Next")
                                    .font(.title3)
                                    .frame(
                                        width: HorizontalSpacingUnit*16,
                                        height: VerticalSpacingUnit*4
                                    )
                                    .focusable()
                            }
                            .controlSize(.large)
                            .keyboardShortcut(.defaultAction)
                            .padding(.vertical, VerticalSpacingUnit)
                        }
                        Spacer()
                    } else if page == 0 {
                        Spacer()
                        HStack {
                            Button(action: {
                                if let onFinish = onFinish {
                                    onFinish()
                                }
                            }) {
                                Text("Skip")
                                    .font(.title3)
                                    .frame(
                                        width: HorizontalSpacingUnit*16,
                                        height: VerticalSpacingUnit*4
                                    )
                            }
                            .controlSize(.large)

                            Button(action: {
                                page += 1
                            }) {
                                Text("Next")
                                    .font(.title3)
                                    .frame(
                                        width: HorizontalSpacingUnit*16,
                                        height: VerticalSpacingUnit*4
                                    )
                            }
                            .controlSize(.large)
                            .keyboardShortcut(.defaultAction)
                        }
                        Spacer()
                    } else if page != lastPage {
                        Spacer()
                        Button(action: {
                            page += 1
                        }) {
                            Text("Next")
                                .font(.title3)
                                .frame(
                                    width: HorizontalSpacingUnit*16,
                                    height: VerticalSpacingUnit*4
                                )
                        }
                        .controlSize(.large)
                        .keyboardShortcut(.defaultAction)
                        Spacer()
                    } else {
                        Button(action: {
                            if let onFinish = onFinish {
                                onFinish()
                            }
                        }) {
                            Text("Let's begin!")
                                .font(.title3)
                                .frame(
                                    width: HorizontalSpacingUnit*16,
                                    height: VerticalSpacingUnit*4
                                )
                                .focusable()
                        }
                        .controlSize(.large)
                        .keyboardShortcut(.defaultAction)
                        .padding(.vertical, VerticalSpacingUnit)
                    }
                }
                Spacer()
            }
            .padding(.vertical, VerticalSpacingUnit)
            .padding(.horizontal, HorizontalSpacingUnit*2)
            .background(.background)
        }
        .padding(.vertical, VerticalSpacingUnit*3)
        .padding(.horizontal, HorizontalSpacingUnit*3)
        .background(OckamDarkerBackground)
        .frame(width: 500)
    }
}


struct GuidedIntro_Previews: PreviewProvider {
    static var previews: some View {
        GuidedIntro(status: .constant(.Disconnected))
            .frame(height: 650)
    }
}
