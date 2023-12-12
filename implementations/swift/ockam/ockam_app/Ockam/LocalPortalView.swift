import SwiftUI

struct LocalPortalView: View {
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
                VStack(alignment: .leading) {
                    Text(verbatim: localService.name)
                        .lineLimit(1)

                    let address =
                        if let scheme = localService.scheme {
                            scheme + "://" + localService.address + ":" + String(localService.port)
                        } else {
                            localService.address + ":" + String(localService.port)
                        }

                    Text(verbatim: address)
                        .foregroundColor(OckamSecondaryTextColor)
                        .font(.caption)
                        .lineLimit(1)
                }
                Spacer()
                Image(systemName: "chevron.right")
                    .rotationEffect(
                        isOpen ? Angle.degrees(90.0) : Angle.degrees(0), anchor: .center)
                    .padding(0)
            }
        }
        .contentShape(Rectangle())
        .padding(.horizontal, HorizontalSpacingUnit)
        .frame(height: VerticalSpacingUnit*5)
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
            VStack(spacing: 0) {
                let address = localService.address + ":" + String(localService.port)
                if let scheme = localService.scheme {
                    let url = scheme + "://" + address
                    ClickableMenuEntry(
                        text: "Open " + url + "…",
                        action: {
                            if let url = URL(string: url) {
                                NSWorkspace.shared.open(url)
                                self.closeWindow()
                            }
                        })
                }
                ClickableMenuEntry(
                    text: "Copy " + address, clicked: "Copied!",
                    action: {
                        copyToClipboard(address)
                        self.closeWindow()
                    })
                ClickableMenuEntry(
                    text: "Share…",
                    action: {
                        OpenWindowWorkaround.shared.openWindow(
                            windowName: "invite-to-portal",
                            value: localService.id
                        )
                        bringInFront()
                        self.closeWindow()
                    })
                ClickableMenuEntry(
                    text: "Delete",
                    action: {
                        delete_local_service(self.localService.name)
                    })
            }
            .padding(.leading, HorizontalSpacingUnit)
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
