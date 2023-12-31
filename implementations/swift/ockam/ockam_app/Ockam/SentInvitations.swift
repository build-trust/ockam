import SwiftUI

struct SentInvitations: View {
    @State private var isHovered = false
    @State private var isOpen = false
    @Binding var localService: LocalService

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 0) {
                Text("Shared with")
                    .font(.body)
                    .padding(.leading, 20 + StandardIconTextSpacing + HorizontalSpacingUnit)
                    .padding(.trailing, HorizontalSpacingUnit)
                Spacer()
                Image(systemName: "chevron.right")
                    .rotationEffect(
                        isOpen ? Angle.degrees(90.0) : Angle.degrees(0), anchor: .center
                    )
                    .padding([.trailing], HorizontalSpacingUnit)
            }
            .frame(height: VerticalSpacingUnit*3.5)
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
            .padding(.horizontal, WindowBorderSize)
            
            if isOpen {
                Divider()
                    .padding(.top, WindowBorderSize)
                HStack(spacing: 0) {
                    ScrollView {
                        VStack(alignment: .leading, spacing: 0) {
                            ForEach(self.$localService.sharedWith) { invitee in
                                SentInvitation(
                                    invitee: invitee,
                                    localServiceName: .constant(localService.name)
                                )
                            }
                        }
                        .padding(.leading, HorizontalSpacingUnit*5)
                    }
                    .scrollIndicators(ScrollIndicatorVisibility.never)
                    .frame(maxHeight: 350)
                    Spacer()
                }
                .padding(.vertical, VerticalSpacingUnit)
                .background(HierarchicalShapeStyle.quinary)
                Divider()
                    .padding(.bottom, WindowBorderSize)
            }
        }
    }
}


struct SentInvitations_Previews: PreviewProvider {
    @State static var state = swift_demo_application_state()
    
    static var previews: some View {
        SentInvitations(
            localService: $state.localServices[0]
        )
        .frame(width: 320, height: 200)
    }
}
