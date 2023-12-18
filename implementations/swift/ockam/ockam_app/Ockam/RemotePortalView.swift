import SwiftUI

struct RemotePortalView: View {
    @Environment(\.presentationMode) var presentationMode: Binding<PresentationMode>
    @Environment(\.openWindow) var openWindow

    @State private var isHovered = false
    @State private var isOpen = false
    @ObservedObject var service: Service

    @State var padding = 0.0

    func closeWindow() {
        self.presentationMode.wrappedValue.dismiss()
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 0) {
                let portalIcon = if #available(macOS 14, *) {
                    "arrow.up.left.arrow.down.right"
                } else {
                    "arrow.up.arrow.down"
                }
                Image(systemName: portalIcon)
                    .frame(width: 20)
                    .font(.system(size: 12, weight: .bold))
                    .padding(.trailing, StandardIconTextSpacing)
                    .opacity((service.enabled && service.available) ? 1.0 : 0.4)

                VStack(alignment: .leading, spacing: 0) {
                    Text(service.sourceName).lineLimit(1)

                    HStack(spacing: 0) {
                        Image(systemName: "circle.fill")
                            .font(.system(size: 7))
                            .foregroundColor( service.enabled ? (service.available ? .green : Color.init(hex: OckamErrorColor)) : .orange
                            )
                            .opacity(0.9)
                            .padding(.top, 1)
                            .padding(.trailing, 4)

                        if !service.enabled {
                            Text(verbatim: "Not connected")
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
                }
                Spacer()
                Image(systemName: "chevron.right")
                    .rotationEffect(
                        isOpen ? Angle.degrees(90.0) : Angle.degrees(0), anchor: .center)
            }
            .padding(.leading, padding)
            .frame(height: VerticalSpacingUnit*4)
            .padding(.horizontal, HorizontalSpacingUnit*2)
            .contentShape(Rectangle())
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

                    if service.enabled {
                        ClickableMenuEntry(
                            text: "Temporarily disconnect",
                            action: {
                                disable_accepted_service(service.id)
                            },
                            textPadding: padding + 35,
                            compact: false
                        )
                    } else {
                        ClickableMenuEntry(
                            text: "Connect to the portal",
                            action: {
                                enable_accepted_service(service.id)
                            },
                            textPadding: padding + 35,
                            compact: false
                        )
                    }
                    ClickableMenuEntry(
                        text: "Delete the portal inlet",
                        action: {
                            OpenWindowWorkaround.shared.openWindow(
                                windowName: "delete-portal-confirmation",
                                value: service.id
                            )
                        },
                        textPadding: padding + 35,
                        compact: false
                    )
                    
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
                                    text: "Open address…",
                                    action: {
                                        if let url = URL(string: url) {
                                            NSWorkspace.shared.open(url)
                                        }
                                    },
                                    textPadding: padding + 35,
                                    compact: false
                                )
                            }
                            ClickableMenuEntry(
                                text: "Copy localhost address", clicked: "Copied!",
                                action: {
                                    copyToClipboard(address)
                                    self.closeWindow()
                                },
                                textPadding: padding + 35,
                                compact: false
                            )
                        }
                    }
                }
                .padding(.horizontal, WindowBorderSize)
                .padding(.vertical, WindowBorderSize)
                .background(HierarchicalShapeStyle.quinary)
                Divider()
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
