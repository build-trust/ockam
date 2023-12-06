
import SwiftUI


struct GuidedIntro: View {
    @Binding var status: OrchestratorStatus
    @State private var page = 0
    @State var onFinish: (() -> Void)? = nil
    let enrollmentPage = 3
    let lastPage = 9

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

                    case 1:
                        Text("What do I use it for?")
                            .font(.title)
                            .padding(.vertical, VerticalSpacingUnit*2)
                        Text(
"""
Let’s say that you have a service, which is a process exposing a TCP server listening on a host and port, on your computer. For example, this might be a web app written in React, or a game written in Javascript that you have running on `localhost:3000`. And you want feedback from your friend on the UI of this web app, or the playability of your game, that you’re working on.

Once you install Ockam.app on your Mac, it allows you to share this service running on your `localhost:3000` to your friend without having to expose your computer to the Internet. It does not require you or them to have to change any network settings or modify any firewall policies on your machine or theirs. You can simply use Ockam.app to create an invitation for your friend to access your service, and email it to them.

Then, once they have Ockam.app installed on their Mac, they can accept this email invitation. And then they can access YOUR service as if it is running on THEIR local machine, on localhost and some port that is available, eg: `localhost:10000`.
"""                     )

                    case 2:
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

"""                     )

                    case 3:
                        Text("Enroll using email & password or GitHub")
                            .font(.title)
                            .padding(.vertical, VerticalSpacingUnit*2)

                        if status == OrchestratorStatus.Disconnected {
                            Text(
    """
    During the enrollment process, your web browser will be launched with a link that will ask you to authenticate.
    """                     )

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

                    case 4:
                        Text("How to create a service")
                            .font(.title)
                            .padding(.vertical, VerticalSpacingUnit*2)

                        Text(
"""
One of the main things that you might want to do with the Ockam.app is create a service, which is a process that is listening on a port.

This service can then be shared with your friends so that they can access it, without exposing your computer to the Internet or having to change any network settings.
"""                     )
                        Image("CreateService01")
                            .resizable()
                            .aspectRatio(contentMode: .fit)
                            .padding(.top, VerticalSpacingUnit)
                            .frame(height: 200)

                    case 5:
                        Text("How to create a service")
                            .font(.title)
                            .padding(.vertical, VerticalSpacingUnit*2)
                        Text(
"""
To create a service, just make note of what IP address (eg: localhost) and port (eg: 3000) that you would like to share with your friends resides at.

This name, along with your email address will be shared with your friends when you send them invitations.
"""
                        )
                        Image("CreateService02")
                            .resizable()
                            .aspectRatio(contentMode: .fit)
                            .padding(.top, VerticalSpacingUnit)
                            .frame(height: 200)

                    case 6:
                        Text("How to share a service")
                            .font(.title)
                            .padding(.vertical, VerticalSpacingUnit*2)
                        Text(
"""
Once you have created a service, as the app instructed you, the next step is to share it with your friend or friends.

In order to do this, click on the Ockam icon in the menu bar, and then click on the service that you want to share.
"""
                        )
                        Image("ShareService01")
                            .resizable()
                            .aspectRatio(contentMode: .fit)
                            .padding(.top, VerticalSpacingUnit)
                            .frame(height: 200)

                    case 7:
                        Text("How to share a service")
                            .font(.title)
                            .padding(.vertical, VerticalSpacingUnit*2)
                        Text(
"""
This will pop open as the main element displayed inside the app. You can then choose “Share” and then type in an email address of your friend.
"""
                        )
                        Image("ShareService02")
                            .resizable()
                            .aspectRatio(contentMode: .fit)
                            .padding(.vertical, VerticalSpacingUnit)
                            .frame(height: 200)

                        Text(
"""
Please note that if your friend is going to enroll into Ockam using their GitHub authentication path, then you have to provide their GitHub primary email address here. Otherwise, they will not be able to access your service using this email address.

Once you click on “Share”, an email will be sent to them by Ockam. And they will be able to enroll, accept your invitation, and then access your service.
"""
                        )
                    case 8:
                        Text("How to access a shared service")
                            .font(.title)
                            .padding(.vertical, VerticalSpacingUnit*2)

                        Text(
"""
Once a friend has shared a service (on their computer) with you, you will get an email invitation letting you know that this has happened. At this point, you can either interact with the invitation via the email or directly in the Ockam.app itself.

If you want to handle the invitation via the app, here are the steps
Once the app is installed, and you have enrolled successfully, the invitations waiting for you will be shown in the app.
"""
                        )

                        Image("AccessService01")
                            .resizable()
                            .aspectRatio(contentMode: .fit)
                            .padding(.vertical, VerticalSpacingUnit)
                            .frame(height: 200)

                    case 9:
                        Text("How to access a shared service")
                            .font(.title)
                            .padding(.vertical, VerticalSpacingUnit*2)

                        Text(
"""
You can click on an invitation to open it. And then you can click on Accept to start using the service. Or Decline the invitation if you don’t want to. If you decline the invitation, and change your mind, you will have to ask you friend to send you another invitation to the same service.
"""
                        )

                        Image("AccessService02")
                            .resizable()
                            .aspectRatio(contentMode: .fit)
                            .padding(.top, VerticalSpacingUnit)
                            .frame(height: 200)

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
                        .keyboardShortcut(.defaultAction)
                        Spacer()
                    } else {
                        Button(action: {
                            if let onFinish = onFinish {
                                onFinish()
                            }
                        }) {
                            Text("Finish")
                                .font(.title3)
                                .frame(
                                    width: HorizontalSpacingUnit*16,
                                    height: VerticalSpacingUnit*4
                                )
                                .focusable()
                        }
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
            .frame(height: 500)
    }
}
