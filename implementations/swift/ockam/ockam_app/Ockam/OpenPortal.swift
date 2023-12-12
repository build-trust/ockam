import SwiftUI

struct OpenPortal: View {
    @Environment(\.presentationMode) var presentationMode: Binding<PresentationMode>
    @FocusState private var isFocused: Bool

    @Binding var localServices: [LocalService]
    @State private var isProcessing = false
    @State private var errorMessage = ""
    @State private var serviceName = ""
    @State private var serviceAddress = "localhost:10000"

    var body: some View {
        VStack(alignment: .leading) {
            Grid(alignment: .leading) {
                GridRow {
                    VStack(alignment: .leading) {
                        Text(verbatim: "Name")
                        Text(verbatim: "A name for your portal").font(.caption)
                    }
                    .padding(.top, 6)
                    TextField("Portal name", text: $serviceName)
                        .focused($isFocused)
                        .onAppear(perform: {
                            isFocused = true
                        })
                }
                GridRow {
                    VStack(alignment: .leading) {
                        Text(verbatim: "Address")
                        Text(verbatim: "The tcp address where your service is running").font(.caption)
                    }
                    .padding(.top, 6)
                    TextField("Address", text: $serviceAddress)
                }
            }
            .padding(10)


            if !errorMessage.isEmpty {
                Text("Error: \(errorMessage)")
                    .foregroundColor(.red)
                    .padding(10)
            }

            Spacer()

            if localServices.isEmpty {
                Hint(
"""
One of the main things that you might want to do with the Ockam.app is open a portal, which allows a TCP service, to be shared with your friends.

Once you open a portal, you can invite your friends to access it, without exposing your computer to the Internet or having to change any network settings.

After you've opened a portal, don't forget to share it with your friends!
"""
                )
            }

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
                        self.errorMessage = ""

                        isProcessing = true
                        let error = create_local_service(
                            self.serviceName,
                            self.serviceAddress
                        )
                        isProcessing = false

                        if error == nil {
                            self.errorMessage = ""
                            self.serviceName = ""
                            self.serviceAddress = "localhost:10000"
                            self.closeWindow()
                        } else {
                            self.errorMessage = String(cString: error.unsafelyUnwrapped)
                        }
                    },
                    label: {
                        Text("Open Portal")
                    }
                )
                .disabled(!canCreateService() && !isProcessing)
                .keyboardShortcut(.defaultAction)
                .padding(10)
            }
            .background(OckamDarkerBackground)
        }
        .frame(width: 600, height: localServices.isEmpty ? 340 : 150)
    }

    func closeWindow() {
        self.presentationMode.wrappedValue.dismiss()
    }

    func canCreateService() -> Bool {
        return !self.serviceName.isEmpty && !self.serviceAddress.isEmpty
    }
}

struct CreateServiceView_Previews: PreviewProvider {
    static var previews: some View {
        OpenPortal(localServices: .constant([]))
    }
}
