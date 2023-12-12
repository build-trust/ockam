
import SwiftUI


class GlobalPage: ObservableObject {
    static let shared = GlobalPage()
    @Published var page = 0
}

struct GuidedIntro: View {
    @Binding var status: OrchestratorStatus

    @State var invitation: InvitationContainer = InvitationContainer.shared
    @State var onEnroll: (() -> Void)? = nil
    @State var onFinish: (() -> Void)? = nil
    @State var canSkip = true
    @State var page = 0
    let enrollmentPage = 2
    let lastPage = 2

    var body: some View {
        Group {
            HStack {
                Spacer()
                VStack(spacing: VerticalSpacingUnit) {
                    switch(page) {
                    case 0:
                        Text("Easily share services with your friends")
                            .font(.title)
                            .padding(.vertical, VerticalSpacingUnit*2)
                            .lineLimit(2)

                        Text(
"""
Ockam.app is a desktop app for macOS. With Ockam.app you can easily share private self-hosted services safely with your friends. It hides these services from the internet.
"""
                        )

                        Image("Diagram")
                            .resizable()
                            .aspectRatio(contentMode: .fit)
                            .frame(height: 300)

                        Text(
"""
Let’s say that you have a TCP service that you can access from your computer. You can share that resource with a friend so they can use it. It is as easy as opening a portal from your computer to your friend’s computer. No need to change any network settings or modify any firewall policies on your machine or theirs.
"""
                        )

                        Spacer()
                        HStack {
                            if canSkip {
                                Button(action: {
                                    // trigger the onFinish directly, so if we have different
                                    // windows also showing the tour they remain consistent
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
                            }

                            Button(action: {
                                GlobalPage.shared.page += 1
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

                        Spacer()
                        Button(action: {
                            GlobalPage.shared.page += 1
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

                    case 2:
                        if status == OrchestratorStatus.Disconnected {
                            Text("Enroll using email & password or GitHub")
                                .font(.title)
                                .padding(.vertical, VerticalSpacingUnit*2)

                            Text(
"""
During the enrollment process, your web browser will be launched with a link that will ask you to authenticate.
"""                         )

                            Image("EnrollmentPage")
                                .resizable()
                                .aspectRatio(contentMode: .fit)
                                .frame(height: 300)

                            EnrollmentStatus(status: $status)

                            Spacer()
                            Button(action: {
                                enroll_user()
                                if let onEnroll = onEnroll {
                                    onEnroll()
                                }
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
                            Spacer()

                        } else if status != OrchestratorStatus.Connecting &&
                                    status != OrchestratorStatus.Connected {
                            Text("Enrolling")
                                .font(.title)
                                .padding(.vertical, VerticalSpacingUnit*2)
                            EnrollmentStatus(status: $status)
                        } else if status == OrchestratorStatus.Connecting ||
                                    status == OrchestratorStatus.Connected {
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

                            Spacer()
                            Button(action: {
                                GlobalPage.shared.page += 1
                            }) {
                                Text("Let's begin!")
                                    .font(.title3)
                                    .frame(
                                        width: HorizontalSpacingUnit*16,
                                        height: VerticalSpacingUnit*4
                                    )
                            }
                            .controlSize(.large)
                            .keyboardShortcut(.defaultAction)
                            Spacer()
                        }

                    default:
                        EmptyView()
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
        .onAppear(perform: {
            if page > lastPage {
                if let onFinish = onFinish {
                    onFinish()
                }
            }
        })
        .onReceive(GlobalPage.shared.$page, perform: { newValue in
            page = newValue
            // when we move the value over the last page
            // we can assume that the user has completed
            if newValue > lastPage {
                if let onFinish = onFinish {
                    onFinish()
                }
            }
        })
    }
}


struct GuidedIntro_Previews: PreviewProvider {
    static var previews: some View {
        GuidedIntro(status: .constant(.Disconnected))
            .frame(height: 650)
    }
}
