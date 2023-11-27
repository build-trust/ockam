import SwiftUI

struct ServiceGroupView: View {
    @ObservedObject var group: ServiceGroup
    @State var back: (() -> Void)? = nil
    @State var action: (() -> Void)? = nil
    @State private var isHovered = false
    @State private var isOpen = false

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: HorizontalSpacingUnit) {
                VStack(alignment: .leading, spacing: 0) {
                    if let name = group.name {
                        Text(verbatim: name).lineLimit(1)
                        Text(verbatim: group.email)
                            .font(.caption)
                            .foregroundColor(OckamSecondaryTextColor)
                            .lineLimit(1)
                    } else {
                        Text(verbatim: group.email)
                            .lineLimit(1)
                    }
                }
                Spacer()
                Circle()
                    .fill(Color.orange)
                    .frame(width: 8, height: 8)
                    .opacity(group.invitations.isEmpty ? 0 : 1)
                    .padding(.trailing, HorizontalSpacingUnit)

                ProfilePicture(url: group.imageUrl, size: 32)
                Image(systemName: "chevron.right")
                    .rotationEffect( isOpen ? Angle(degrees: 90) : Angle(degrees: 0))
            }
        }
        .contentShape(Rectangle())
        .onHover { hover in
            isHovered = hover
        }
        .padding(.horizontal, HorizontalSpacingUnit)
        .frame(height: VerticalSpacingUnit*5)
        .background(isHovered ? Color.gray.opacity(0.25) : Color.clear)
        .onTapGesture {
            withAnimation {
                isOpen = !isOpen

                if isOpen {
                    if let action = self.action {
                        action()
                    }
                } else {
                    if let back = self.back {
                        back()
                    }
                }
            }
            // for some reason hover doesn't change back to false
            // when out of view
            isHovered = false
        }
        .cornerRadius(4)

        if isOpen {
            Group {
                ForEach(group.invitations) { invite in
                    IncomingInvite(invite: invite)
                }
                ForEach(group.incomingServices) { service in
                    RemoteServiceView(service: service)
                }
            }
            .padding(.leading, VerticalSpacingUnit*2)
        }
    }
}


struct ServiceGroupView_Previews: PreviewProvider {
    @State static var state = swift_demo_application_state()

    static var previews: some View {
        VStack(spacing: 0){
            ServiceGroupView(group: state.groups[1])
            ServiceGroupView(group: state.groups[2])
        }
        .frame(width: 320, height: 200)
    }
}
