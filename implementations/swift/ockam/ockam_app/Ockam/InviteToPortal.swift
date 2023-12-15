import SwiftUI

struct InviteToPortal: View {
    @Environment(\.presentationMode) var presentationMode: Binding<PresentationMode>

    @Binding var state_loaded: Bool
    @State var isProcessing = false
    @State public var localService: LocalService
    @State var emails = Set<String>()
    @State var errorMessage = ""

    var body: some View {
        VStack(alignment: .center) {
            Text("Invite your friends to access: **\(localService.name)**")
                .font(.title)
                .padding(.top, VerticalSpacingUnit)

            EmailListView(emailList: $emails)

            if !errorMessage.isEmpty {
                Text("Error: \(errorMessage)")
                    .foregroundColor(.red)
            }

            Spacer()

            Hint(
"""
Here we will add a list of email addresses to invite to this Portal.

Once your friends accept their invitation, the '\(localService.name)' service is shared securely over an end-to-end encrypted Ockam Portal. They will have access to it on their localhost!
"""
            )
            .frame(height: 130)


            HStack {
                Spacer()
                Button(
                    action: {
                        self.closeWindow()
                    },
                    label: {
                        Text("Cancel")
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
                        Text("Invite to Portal")
                    }
                )
                .disabled(!canShareService() && !isProcessing)
                .keyboardShortcut(.defaultAction)
                .padding(10)
            }
            .background(OckamDarkerBackground)
        }
        .frame(width: 600, height: 400)
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
            )
        )
        .frame(height: 400)
    }
}
