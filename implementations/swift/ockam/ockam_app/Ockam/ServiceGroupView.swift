import SwiftUI

struct ServiceGroupView: View {
    @ObservedObject var group: ServiceGroup
    @State var back: (() -> Void)? = nil
    @State var action: (() -> Void)? = nil
    @State private var isHovered = false
    @State private var isOpen = false
    
    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 0) {
                Image(systemName: "shared.with.you")
                    .frame(width: 20)
                    .font(.system(size: 12, weight: .bold))
                    .padding(.trailing, StandardIconTextSpacing)
                
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
                        
                        let subtitle =
                        if group.invitations.isEmpty {
                            if group.incomingServices.count == 1 {
                                "\(group.incomingServices.count) portal accessible"
                            } else {
                                "\(group.incomingServices.count) portals accessible"
                            }
                        } else {
                            if group.incomingServices.count == 1 {
                                if group.invitations.count == 1 {
                                    "\(group.incomingServices.count) portal accessible, \(group.invitations.count) invitation"
                                } else {
                                    "\(group.incomingServices.count) portal accessible, \(group.invitations.count) invitations"
                                }
                            } else {
                                if group.invitations.count == 1 {
                                    "\(group.incomingServices.count) portals accessible, \(group.invitations.count) invitation"
                                } else {
                                    "\(group.incomingServices.count) portals accessible, \(group.invitations.count) invitations"
                                }
                            }
                        }
                        
                        Text(verbatim: subtitle)
                            .font(.caption)
                            .foregroundColor(OckamSecondaryTextColor)
                            .lineLimit(1)
                    }
                }
                Spacer()
                Circle()
                    .fill(Color.orange)
                    .frame(width: 8, height: 8)
                    .opacity(group.invitations.isEmpty ? 0 : 1)
                    .padding(.trailing, HorizontalSpacingUnit)
                
                ProfilePicture(
                    url: group.imageUrl,
                    size: 28,
                    placeholder: ""
                )
                
                Image(systemName: "chevron.right")
                    .rotationEffect( isOpen ? Angle(degrees: 90) : Angle(degrees: 0))
            }
            .contentShape(Rectangle())
            .onHover { hover in
                isHovered = hover
            }
            .padding(.horizontal, HorizontalSpacingUnit)
            .frame(height: VerticalSpacingUnit*4)
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
            .background( isHovered ?
                         AnyShapeStyle(HierarchicalShapeStyle.quaternary) :
                            AnyShapeStyle(Color.clear)
            )
            .cornerRadius(4)
            .padding(.horizontal, WindowBorderSize)
            .padding(.vertical, WindowBorderSize)
            
            if isOpen {
                VStack(spacing: 0) {
                    Divider()
                    ForEach(group.invitations) { invite in
                        IncomingInvite(
                            invite: invite,
                            padding: HorizontalSpacingUnit*2
                        )
                    }
                    ForEach(group.incomingServices) { service in
                        RemotePortalView(
                            service: service,
                            padding: HorizontalSpacingUnit*2
                        )
                    }
                }.background(HierarchicalShapeStyle.quinary)
            }
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
