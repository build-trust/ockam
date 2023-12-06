import SwiftUI

struct RemotePortalView: View {
    @Environment(\.presentationMode) var presentationMode: Binding<PresentationMode>
    @Environment(\.openWindow) var openWindow

    @State private var isHovered = false
    @State private var isOpen = false
    @ObservedObject var service: Service

    func closeWindow() {
        self.presentationMode.wrappedValue.dismiss()
    }

    var body: some View {
        VStack(alignment: .leading) {
            HStack {
                Image(systemName: "circle")
                    .foregroundColor(
                        service.enabled ? (service.available ? .green : .red) : .orange
                    )
                    .frame(maxWidth: 16, maxHeight: 16)

                VStack(alignment: .leading) {
                    Text(service.sourceName).font(.title3).lineLimit(1)
                    if !service.enabled {
                        Text(verbatim: "Disconnected")
                            .foregroundStyle(OckamSecondaryTextColor)
                            .font(.caption)
                    } else {
                        if service.available {
                            let address =
                            if let scheme = service.scheme {
                                scheme + "://" + service.address.unsafelyUnwrapped + ":"
                                + String(service.port.unsafelyUnwrapped)
                            } else {
                                service.address.unsafelyUnwrapped + ":"
                                + String(service.port.unsafelyUnwrapped)
                            }
                            Text(verbatim: address)
                                .foregroundStyle(OckamSecondaryTextColor)
                                .font(.caption)
                                .lineLimit(1)
                        } else {
                            Text(verbatim: "Connecting")
                                .foregroundStyle(OckamSecondaryTextColor)
                                .font(.caption)
                        }
                    }
                }
                Spacer()
                Image(systemName: "chevron.right")
                    .rotationEffect(
                        isOpen ? Angle.degrees(90.0) : Angle.degrees(0), anchor: .center)
            }
            .frame(height: VerticalSpacingUnit*5)
            .padding(.horizontal, HorizontalSpacingUnit)
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
                VStack(spacing: 0) {
                    if service.available {
                        if service.enabled {
                            let address =
                            service.address.unsafelyUnwrapped + ":"
                            + String(service.port.unsafelyUnwrapped)
                            if let scheme = service.scheme {
                                let url =
                                scheme + "://" + service.address.unsafelyUnwrapped + ":"
                                + String(service.port.unsafelyUnwrapped)
                                ClickableMenuEntry(
                                    text: "Open " + url + "…",
                                    action: {
                                        if let url = URL(string: url) {
                                            NSWorkspace.shared.open(url)
                                        }
                                    })
                            }
                            ClickableMenuEntry(
                                text: "Copy " + address, clicked: "Copied!",
                                action: {
                                    copyToClipboard(address)
                                    self.closeWindow()
                                })
                        }
                    }

                    if service.enabled {
                        ClickableMenuEntry(
                            text: "Disconnect",
                            action: {
                                disable_accepted_service(service.id)
                            })
                    } else {
                        ClickableMenuEntry(
                            text: "Connect",
                            action: {
                                enable_accepted_service(service.id)
                            })
                    }
                    ClickableMenuEntry(
                        text: "Delete",
                        action: {
                            openWindow(id: "delete-portal-confirmation", value: service.id)
                        })
                }
                .padding(.leading, HorizontalSpacingUnit*2)
            }
        }
    }
}


struct RemoteServiceView_Previews: PreviewProvider {
    @State static var state = swift_demo_application_state()

    static var previews: some View {
        VStack {
            ForEach(state.groups[0].incomingServices) { service in
                RemotePortalView(service: service)
            }
            ForEach(state.groups[1].incomingServices) { service in
                RemotePortalView(service: service)
            }
            ForEach(state.groups[2].incomingServices) { service in
                RemotePortalView(service: service)
            }
        }
        .frame(width: 300, height: 600)
    }
}
