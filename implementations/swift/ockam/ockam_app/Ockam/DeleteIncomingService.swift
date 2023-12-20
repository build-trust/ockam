import SwiftUI

struct DeleteIncomingPortalView: View {
    @Environment(\.presentationMode) var presentationMode: Binding<PresentationMode>
    @State var service: Service

    var body: some View {
        VStack {
            Spacer()
            Text(
                "This will permanently delete '\(service.sourceName)'. If you need access to it in the future, your friend would have to invite you again." +
                "\n\nAre you sure you want to delete: '\(service.sourceName)'?"
            )
            .padding()
            Spacer()
            HStack {
                Spacer()
                Button(
                    action: {
                        print("ignoring: \(service.id)")
                        ignore_invitation(service.id)
                        self.closeWindow()
                    },
                    label: {
                        Text("Delete")
                    }
                )
                Button(
                    action: {
                        self.closeWindow()
                    },
                    label: {
                        Text("Cancel")
                    }
                )
                .keyboardShortcut(.defaultAction)
                .padding(10)
            }
            .background(OckamDarkerBackground)
        }
        .frame(width: 300, height: 200)
    }

    func closeWindow() {
        presentationMode.wrappedValue.dismiss()
    }
}


struct DeleteIncomingPortalView_Previews: PreviewProvider {
    @State static var state = swift_demo_application_state()

    static var previews: some View {
        DeleteIncomingPortalView(service: state.groups[1].incomingServices[0])
    }
}
