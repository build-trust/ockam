import SwiftUI

struct CreateServiceView: View {
    @Environment(\.presentationMode) var presentationMode: Binding<PresentationMode>
    @FocusState private var isFocused: Bool

    @Binding var state_loaded: Bool
    @State var isProcessing = false
    @State var errorMessage = ""
    @State var serviceName = ""
    @State var serviceAddress = "localhost:10000"

    var body: some View {
        VStack(alignment: .leading) {
            Grid(alignment: .leading) {
                GridRow {
                    VStack(alignment: .leading) {
                        Text(verbatim: "Name")
                        Text(verbatim: "A name for your service").font(.caption)
                    }
                    .padding(.top, 6)
                    TextField("Service name", text: $serviceName)
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

            //use opacity to pre-allocate the space for this component
            Text("Error: \(errorMessage)")
                .opacity(errorMessage.isEmpty ? 0 : 1)
                .foregroundColor(.red)
                .padding(10)

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
                        Text("Create Service")
                    }
                )
                .disabled(!canCreateService() && !isProcessing)
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

    func canCreateService() -> Bool {
        return !self.serviceName.isEmpty && !self.serviceAddress.isEmpty && state_loaded
    }
}

struct CreateServiceView_Previews: PreviewProvider {
    static var previews: some View {
        CreateServiceView(state_loaded: .constant(true))
    }
}
