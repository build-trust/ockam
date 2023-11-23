import SwiftUI

struct IgnoreServiceView: View {
    @Environment(\.presentationMode) var presentationMode: Binding<PresentationMode>
    @State var service: Service

    var body: some View {
        VStack {
            Text(
                "Once you click Ignore, the service '\(service.sourceName)' will no longer show up here.\n" +
                "Are you sure you want to do this?\n" +
                "Once ignored, the only way to get this back is to have the person who sent you the invite, to send another one."
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
                        Text("Ignore")
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
            .background(.black.opacity(0.1))
        }
        .frame(width: 300, height: 180)
    }

    func closeWindow() {
        presentationMode.wrappedValue.dismiss()
    }
}


struct IgnoreServiceView_Previews: PreviewProvider {
    @State static var state = swift_demo_application_state()

    static var previews: some View {
        IgnoreServiceView(service: state.groups[1].incomingServices[0])
    }
}
