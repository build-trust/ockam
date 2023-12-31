import SwiftUI

public struct SentInvitation: View {
    @State private var isHovered = false
    @State private var pressedButton = ""
    @Binding var invitee: Invitee
    @Binding var localServiceName: String

    public var body: some View {
        HStack(spacing: 0) {
            @State var isEmailHovered = false

            Text(invitee.email)
                .frame(height: VerticalSpacingUnit*3)

            Spacer()

            if isHovered && pressedButton == "" {
                Button {
                    pressedButton = "revoke"
                    DispatchQueue.main.async(execute: {
                        revoke_access_local_service(
                            localServiceName,
                            invitee.email
                        )
                    })
                } label: {
                    Text("Revoke")
                }
                .buttonStyle(.plain)
                .frame(height: VerticalSpacingUnit*3)
                .padding(.horizontal, HorizontalSpacingUnit)
                .background(
                    UnevenRoundedRectangle(
                        topLeadingRadius: 5,
                        bottomLeadingRadius: 5,
                        bottomTrailingRadius: 0,
                        topTrailingRadius: 0
                    )
                    .fill(.quinary)
                )
                .padding(.trailing, 1)

                Button {
                    pressedButton = "re-invite"
                    DispatchQueue.main.async(execute: {
                        share_local_service(
                            localServiceName,
                            invitee.email
                        )
                    })
                } label: {
                    Text("Re-Invite")
                }
                .buttonStyle(.plain)
                .frame(height: VerticalSpacingUnit*3)
                .padding(.horizontal, HorizontalSpacingUnit)
                .background(
                    UnevenRoundedRectangle(
                        topLeadingRadius: 0,
                        bottomLeadingRadius: 0,
                        bottomTrailingRadius: 5,
                        topTrailingRadius: 5
                    )
                    .fill(.quinary)
                )
            }

            if pressedButton == "revoke" {
                Text("Revocation issued")
            }

            if pressedButton == "re-invite" {
                Text("Invitation sent")
            }
        }
        .frame(height: VerticalSpacingUnit*3.5)
        .onHover(perform: { hovering in
            isHovered = hovering
            pressedButton = ""
        })
    }
}

struct SentInvitation_Preview: PreviewProvider {
    static var previews: some View {
        SentInvitation(
            invitee: .constant(Invitee(
                name: "name",
                email: "email@example.com"
            )),
            localServiceName: .constant("portal-name")
        )
        .frame(width: 320, height: 200)
    }
}
