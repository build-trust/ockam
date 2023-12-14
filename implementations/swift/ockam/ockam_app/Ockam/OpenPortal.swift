import SwiftUI

struct OpenPortal: View {
    @Environment(\.presentationMode) var presentationMode: Binding<PresentationMode>
    @EnvironmentObject private var appDelegate: AppDelegate
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
                        Text(verbatim: "The TCP address where your service is running").font(.caption)
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
Before you can privately share a TCP or HTTP service with your friends, you have to open a Portal Outlet to it. And then, invite your friends to the Portal.

Please input the name of your portal and the IP:Port of your service.
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
        .frame(width: 600, height: localServices.isEmpty ? 300 : 150)
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
