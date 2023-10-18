/*
    This file aggregate all service-related specific views such as local services, remote services and invitations
*/

import SwiftUI

struct ServiceGroupView: View {
    @ObservedObject var group: ServiceGroup
    @State var back: (() -> Void)? = nil
    @State private var isPictureHovered = false

    var body: some View {
        VStack {
            HStack {
                Image(systemName: "chevron.backward")
                    .frame(width: 32, height: 32)
                    //use opacity to account for space
                    .opacity(isPictureHovered ? 1 : 0)

                Spacer()
                ProfilePicture(url: group.imageUrl, size: 32)
                VStack(alignment: .leading) {
                    if let name = group.name {
                        Text(verbatim: name)
                    }
                    Text(verbatim: group.email)
                }
                Spacer()
                // to match the 32px icon on the left and keep the content centered
                Spacer().frame(width: 32, height: 32)
            }
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .background(isPictureHovered ? Color.gray.opacity(0.25) : Color.clear)
            .buttonStyle(PlainButtonStyle())
            .cornerRadius(4)
            .contentShape(Rectangle())
            .onTapGesture {
                if let back = self.back {
                    back()
                }
            }
            .onHover(perform: { hovering in
                isPictureHovered = hovering
            })
            ForEach(group.invitations) { invite in
                IncomingInvite(invite: invite)
            }
            ForEach(group.incomingServices) { service in
                RemoteServiceView(service: service)
            }
        }
    }
}

struct ServiceGroupButton: View {
    @State private var isHovered = false
    @ObservedObject var group: ServiceGroup
    @State var action: (() -> Void)? = nil

    var body: some View {
        HStack {
            ProfilePicture(url: group.imageUrl, size: 32)
            VStack(alignment: .leading) {
                if let name = group.name {
                    Text(verbatim: name).lineLimit(1)
                }
                Text(verbatim: group.email).lineLimit(1)
            }
            Spacer()
            Image(systemName: "chevron.right")
                .frame(width: 32, height: 32)
        }.onHover { hover in
            isHovered = hover
        }
        .padding(3)
        .background(isHovered ? Color.gray.opacity(0.25) : Color.clear)
        .onTapGesture {
            if let action = self.action {
                action()
            }
            // for some reason hover doesn't change back to false
            // when out of view
            isHovered = false
        }
        .contentShape(Rectangle())
        .cornerRadius(4)
    }
}

struct RemoteServiceView: View {
    @State private var isHovered = false
    @State private var isOpen = false
    @ObservedObject var service: Service

    @Environment(\.presentationMode) var presentationMode: Binding<PresentationMode>
    func closeWindow() {
        self.presentationMode.wrappedValue.dismiss()
    }

    var body: some View {
        VStack(alignment: .leading) {
            HStack {
                Image(systemName: "circle")
                    .foregroundColor(service.available ? ( service.enabled ? .green : .orange) : .red)
                    .frame(maxWidth: 16, maxHeight: 16)

                VStack(alignment: .leading) {
                    Text(service.sourceName).font(.title3).lineLimit(1)
                    if service.available {
                        let address = if let scheme = service.scheme {
                            scheme + "://" + service.address.unsafelyUnwrapped + ":" + String(service.port.unsafelyUnwrapped)
                        } else {
                            service.address.unsafelyUnwrapped + ":" + String(service.port.unsafelyUnwrapped)
                        }
                        Text(verbatim: address).font(.caption).lineLimit(1)
                    } else {
                        Text(verbatim: "Connecting...").font(.caption)
                    }
                }
                Spacer()
                if service.available {
                    Image(systemName: "chevron.right")
                        .frame(width: 32, height: 32)
                        .rotationEffect(isOpen ? Angle.degrees(90.0) : Angle.degrees(0), anchor: .center)
                }
            }
            .padding(3)
            .contentShape(Rectangle())
            .onTapGesture {
                if service.available {
                    withAnimation {
                        isOpen = !isOpen
                    }
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
                            let address = service.address.unsafelyUnwrapped + ":" + String(service.port.unsafelyUnwrapped);
                            if let scheme = service.scheme {
                                let url = scheme + "://" + service.address.unsafelyUnwrapped + ":" + String(service.port.unsafelyUnwrapped)
                                ClickableMenuEntry(text: "Open "+url, action: {
                                    if let url = URL(string: url) {
                                        NSWorkspace.shared.open(url)
                                    }
                                })
                            }
                            ClickableMenuEntry(text: "Copy " + address, clicked: "Copied!", action: {
                                copyToClipboard(address)
                                self.closeWindow()
                            })
                            ClickableMenuEntry(text: "Disconnect", action: {
                                disable_accepted_service(service.id)
                            })
                        } else {
                            ClickableMenuEntry(text: "Connect", action: {
                                enable_accepted_service(service.id)
                            })
                        }
                    }
                }
            }
        }
    }
}

struct LocalServiceView: View {
    @Environment(\.openWindow) private var openWindow

    @State private var isHovered = false
    @State private var isOpen = false
    @ObservedObject var localService: LocalService

    @Environment(\.presentationMode) var presentationMode: Binding<PresentationMode>
    func closeWindow() {
        self.presentationMode.wrappedValue.dismiss()
    }

    var body: some View {
        VStack(alignment: .leading) {
            HStack {
                Image(systemName: "circle")
                    .foregroundColor(localService.available ? .green : .red)
                    .frame(maxWidth: 16, maxHeight: 16)
                VStack(alignment: .leading) {
                    Text(verbatim: localService.name).font(.title3).lineLimit(1)
                    let address = if let scheme = localService.scheme {
                        scheme + "://" + localService.address + ":" + String(localService.port)
                    } else {
                        localService.address + ":" + String(localService.port)
                    }
                    Text(verbatim: address).font(.caption).lineLimit(1)
                }
                Spacer()
                Image(systemName: "chevron.right")
                    .frame(width: 32, height: 32)
                    .rotationEffect(isOpen ? Angle.degrees(90.0) : Angle.degrees(0), anchor: .center)
            }
            .padding(3)
            .contentShape(Rectangle())
            .onTapGesture {
                withAnimation {
                    isOpen = !isOpen
                }            }
            .onHover { hover in
                isHovered = hover
            }
            .background(isHovered ? Color.gray.opacity(0.25) : Color.clear)
            .cornerRadius(4)

            if isOpen {
                VStack(spacing: 0) {
                    let address = localService.address + ":" + String(localService.port);
                    if let scheme = localService.scheme {
                        let url = scheme + "://" + address
                        ClickableMenuEntry(text: "Open "+url, action: {
                            if let url = URL(string: url) {
                                NSWorkspace.shared.open(url)
                                self.closeWindow()
                            }
                        })
                    }
                    ClickableMenuEntry(text: "Copy " + address, clicked: "Copied!", action: {
                        copyToClipboard(address)
                        self.closeWindow()
                    })
                    ClickableMenuEntry(text: "Share", action: {
                        openWindow(id:"share-service", value: localService.id)
                        bringInFront()
                        self.closeWindow()
                    })
                    ClickableMenuEntry(text: "Delete", action: {
                        delete_local_service(self.localService.name)
                    })
                }
            }
        }
    }
}

struct IncomingInvite: View {
    @State private var isHovered = false
    @State private var isOpen = false
    @ObservedObject var invite: Invitation

    var body: some View {
        VStack(alignment: .leading) {
            HStack {
                Image(systemName: invite.accepting ? "envelope.open" : "envelope")
                    .frame(maxWidth: 16, maxHeight: 16)
                VStack(alignment: .leading) {
                    Text(verbatim: invite.serviceName).font(.title3).lineLimit(1)
                    if invite.accepting {
                        Text(verbatim: "Accepting...").font(.caption)
                    } else {
                        if let scheme = invite.serviceScheme {
                            Text(verbatim: scheme).font(.caption)
                        }
                    }
                }
                Spacer()
                if !invite.accepting {
                    Image(systemName: "chevron.right")
                        .rotationEffect(isOpen ? Angle.degrees(90.0) : Angle.degrees(0), anchor: .center)
                        .padding([.trailing], 10)
                }
            }
            .padding(3)
            .contentShape(Rectangle())
            .onTapGesture {
                withAnimation {
                    if !invite.accepting {
                        isOpen = !isOpen
                    }
                }
            }
            .onHover { hover in
                isHovered = hover
            }
            .background(isHovered ? Color.gray.opacity(0.25) : Color.clear)
            .cornerRadius(4)

            if isOpen {
                VStack(spacing: 0) {
                    ClickableMenuEntry(text: "Accept", action: {
                        accept_invitation(invite.id)
                        isOpen = false
                    })
                }
            }
        }
    }
}


struct SentInvitations: View {
    @State private var isHovered = false
    @State private var isOpen = false
    @ObservedObject var state: ApplicationState

    var body: some View {
        VStack(alignment: .leading) {
            HStack {
                Text("Sent Invitations")
                    .font(.body).bold().foregroundColor(.primary.opacity(0.7))
                Spacer()
                Image(systemName: "chevron.right")
                    .rotationEffect(isOpen ? Angle.degrees(90.0) : Angle.degrees(0), anchor: .center)
                    .padding([.trailing], 10)
            }
            .padding(3)
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
                ForEach(state.sent_invitations){ invitation in
                    Text(invitation.email)
                }
            }
        }
    }
}
