import SwiftUI

struct OpenPortal: View {
    @Environment(\.presentationMode) var presentationMode: Binding<PresentationMode>
    @EnvironmentObject private var appDelegate: AppDelegate
    @FocusState private var isFocused: Bool

    @Binding var localServices: [LocalService]
    @State private var isProcessing = false
    @State private var errorMessage = ""
    @State private var serviceName = ""
    @State private var serviceAddress = ""

    var body: some View {
        VStack(alignment: .leading) {
            Grid(alignment: .leading) {
                GridRow {
                    VStack(alignment: .leading) {
                        Text(verbatim: "Name")
                        Text(verbatim: "This is the name your friends will see.").font(.caption)
                    }
                    .padding(.top, 6)
                    TextField("ex: My Web App", text: $serviceName)
                        .focused($isFocused)
                        .onAppear(perform: {
                            isFocused = true
                        })
                }
                GridRow {
                    VStack(alignment: .leading) {
                        Text(verbatim: "Address")
                        Text(verbatim: "The TCP address where your service is running").font(.caption)
                    }
                    .padding(.top, 6)
                    TextField("ex: localhost:3333 or 192.168.1.6:4444 or my-nas:5555", text: $serviceAddress)
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
Here we will pick the TCP or HTTP service that you want to share with your friends. After you click 'Open Portal', invite your friends to this Portal from the main application menu.
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
                            self.serviceAddress = ""
                            self.closeWindow()
                            appDelegate.showPopover()
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
        .frame(width: 600, height: localServices.isEmpty ? 280 : 150)
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
