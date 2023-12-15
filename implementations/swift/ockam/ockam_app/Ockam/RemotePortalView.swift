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
            HStack(spacing: HorizontalSpacingUnit) {
                Image(systemName: "circle")
                    .foregroundColor(
                        service.enabled ? (service.available ? .green : .red) : .orange
                    )
                    .padding(.trailing, StandardIconTextSpacing)

                VStack(alignment: .leading, spacing: 0) {
                    Text(service.sourceName).lineLimit(1)
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
            .padding(.leading, padding)
            .frame(height: VerticalSpacingUnit*4)
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
            .background( isHovered ?
                AnyShapeStyle(HierarchicalShapeStyle.quaternary) :
                AnyShapeStyle(Color.clear)
            )
            .cornerRadius(4)
            .padding(.horizontal, WindowBorderSize)
            .padding(.vertical, 4)

            if isOpen {
                Divider()
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
                                    text: "Open address…",
                                    action: {
                                        if let url = URL(string: url) {
                                            NSWorkspace.shared.open(url)
                                        }
                                    },
                                    textPadding: padding + HorizontalSpacingUnit*2
                                )
                            }
                            ClickableMenuEntry(
                                text: "Copy", clicked: "Copied!",
                                action: {
                                    copyToClipboard(address)
                                    self.closeWindow()
                                },
                                textPadding: padding + HorizontalSpacingUnit*2
                            )
                        }
                    }

                    if service.enabled {
                        ClickableMenuEntry(
                            text: "Disconnect",
                            action: {
                                disable_accepted_service(service.id)
                            },
                            textPadding: padding + HorizontalSpacingUnit*2
                        )
                    } else {
                        ClickableMenuEntry(
                            text: "Connect",
                            action: {
                                enable_accepted_service(service.id)
                            },
                            textPadding: padding + HorizontalSpacingUnit*2
                        )
                    }
                    ClickableMenuEntry(
                        text: "Delete",
                        action: {
                            OpenWindowWorkaround.shared.openWindow(
                                windowName: "delete-portal-confirmation",
                                value: service.id
                            )
                        },
                        textPadding: padding + HorizontalSpacingUnit*2
                    )
                }
                .padding(.horizontal, WindowBorderSize)
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
