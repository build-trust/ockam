import SwiftUI

struct InviteToPortal: View {
    @Environment(\.presentationMode) var presentationMode: Binding<PresentationMode>

    @Binding var state_loaded: Bool
    @State var isProcessing = false
    @State public var localService: LocalService
    @State var emails = Set<String>()
    @State var errorMessage = ""

    var body: some View {
        VStack(alignment: .leading) {
            EmailListView(emailList: $emails)

            //use opacity to pre-allocate the space for this component
            Text("Error: \(errorMessage)")
                .opacity(errorMessage.isEmpty ? 0 : 1)
                .foregroundColor(.red)

            HStack {
                Spacer()
                Button(
                    action: {
                        self.closeWindow()
                    },
                    label: {
                        Text("Close")
                    })
                Button(
                    action: {
                        let emails = Array(self.emails).joined(separator: ";")

                        isProcessing = true
                        let error = share_local_service(
                            localService.name,
                            emails
                        )
                        isProcessing = false

                        if error == nil {
                            self.errorMessage = ""
                            self.closeWindow()
                        } else {
                            self.errorMessage = String(cString: error.unsafelyUnwrapped)
                        }
                    },
                    label: {
                        Text("Invite")
                    }
                )
                .disabled(!canShareService() && !isProcessing)
                .keyboardShortcut(.defaultAction)
                .padding(10)
            }
            .background(OckamDarkerBackground)
        }
        .frame(width: 600)
    }

    func closeWindow() {
        self.presentationMode.wrappedValue.dismiss()
    }

    func canShareService() -> Bool {
        return !self.emails.isEmpty && state_loaded
    }
}

struct ShareServiceView_Previews: PreviewProvider {
    static var previews: some View {
        InviteToPortal(
            state_loaded: .constant(true),
            localService: LocalService(
                name: "my-service",
                address: "127.0.0.1",
                port: 1234, scheme: nil,
                sharedWith: [],
                available: false
            ))
    }
}
