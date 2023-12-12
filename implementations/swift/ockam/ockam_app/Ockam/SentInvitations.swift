import SwiftUI

struct SentInvitations: View {
    @State private var isHovered = false
    @State private var isOpen = false
    @ObservedObject var state: ApplicationState

    var body: some View {
        VStack(alignment: .leading, spacing: VerticalSpacingUnit) {
            HStack(spacing: HorizontalSpacingUnit) {
                Text("Sent invitations")
                    .font(.body)
                    .padding(.horizontal, HorizontalSpacingUnit)
                Spacer()
                Image(systemName: "chevron.right")
                    .rotationEffect(
                        isOpen ? Angle.degrees(90.0) : Angle.degrees(0), anchor: .center
                    )
                    .padding([.trailing], HorizontalSpacingUnit)
            }
            .frame(height: VerticalSpacingUnit*3)
            .contentShape(Rectangle())
            .onTapGesture {
                withAnimation {
                    isOpen = !isOpen
                }
            }
            .onHover { hover in
                isHovered = hover
            }
            .background(isHovered ? Color.gray.opacity(0.25) : Color.clear)
            .cornerRadius(4)

            if isOpen {
                Group {
                    ForEach(state.sent_invitations) { invitation in
                        Text(invitation.email)
                    }
                }
                .padding(.horizontal, HorizontalSpacingUnit*2)
            }
        }
    }
}


struct SentInvitations_Previews: PreviewProvider {
    @State static var state = swift_demo_application_state()

    static var previews: some View {
        SentInvitations(state: state)
            .frame(width: 320, height: 200)
    }
}
