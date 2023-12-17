import SwiftUI

struct LocalPortalView: View {
    @Environment(\.colorScheme) var colorScheme
    @Environment(\.openWindow) private var openWindow
    
    @State private var isHovered = false
    @State private var isOpen = false
    @ObservedObject var localService: LocalService
    
    @Environment(\.presentationMode) var presentationMode: Binding<PresentationMode>
    
    func closeWindow() {
        self.presentationMode.wrappedValue.dismiss()
    }
    
    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 0) {
                Image(systemName: "rectangle.stack")
                    .frame(width: 20)
                    .font(.system(size: 12, weight: .bold))
                    .padding(.trailing, StandardIconTextSpacing)

                VStack(alignment: .leading, spacing: 0) {
                    Text(verbatim: localService.name)
                        .lineLimit(1)

                    let address =
                    if let scheme = localService.scheme {
                        scheme + "://" + localService.address + ":" + String(localService.port)
                    } else {
                        localService.address + ":" + String(localService.port)
                    }

                    HStack(spacing: 0) {
                        Image(systemName: "circle.fill")
                            .font(.system(size: 7))
                            .foregroundColor(.green)
                            .opacity(0.9)
                            .padding(.top, 1)
                            .padding(.trailing, 4)

                        Text(verbatim: address)
                            .foregroundColor(OckamSecondaryTextColor)
                            .font(.caption)
                            .lineLimit(1)
                    }
                }
                Spacer()
                Image(systemName: "chevron.right")
                    .rotationEffect(
                        isOpen ? Angle.degrees(90.0) : Angle.degrees(0), anchor: .center)
                    .padding(0)
            }
            .contentShape(Rectangle())
            .padding(.horizontal, HorizontalSpacingUnit)
            .frame(height: VerticalSpacingUnit*4)
            .onTapGesture {
                withAnimation {
                    isOpen = !isOpen
                }
            }
            .onHover { hover in
                isHovered = hover
            }
            .background( isHovered ?
                         AnyShapeStyle(HierarchicalShapeStyle.quaternary) :
                            AnyShapeStyle(Color.clear)
            )
            .cornerRadius(4)
            .padding(.horizontal, WindowBorderSize)
            .padding(.vertical, WindowBorderSize)
            
            if isOpen {
                Divider()
                VStack(spacing: 0) {
                    let address = localService.address + ":" + String(localService.port)
                    if let scheme = localService.scheme {
                        let url = scheme + "://" + address
                        ClickableMenuEntry(
                            text: "Open address…",
                            action: {
                                if let url = URL(string: url) {
                                    NSWorkspace.shared.open(url)
                                    self.closeWindow()
                                }
                            },
                            textPadding: HorizontalSpacingUnit*2
                        )
                    }
                    ClickableMenuEntry(
                        text: "Invite a friend…",
                        action: {
                            OpenWindowWorkaround.shared.openWindow(
                                windowName: "invite-to-portal",
                                value: localService.id
                            )
                            bringInFront()
                        },
                        textPadding: 27
                    )
                    ClickableMenuEntry(
                        text: "Close the portal",
                        action: {
                            delete_local_service(self.localService.name)
                        },
                        textPadding: 27
                    )
                }
                .padding(.horizontal, WindowBorderSize)
                .padding(.vertical, WindowBorderSize)
                .background(HierarchicalShapeStyle.quinary)
                Divider()
            }
        }
    }
}


struct LocalServiceView_Previews: PreviewProvider {
    @State static var state = swift_demo_application_state()
    
    static var previews: some View {
        VStack(spacing: 0) {
            LocalPortalView(localService: state.localServices[0])
            LocalPortalView(localService: state.localServices[1])
        }
        .frame(width: 320, height: 200)
    }
}
