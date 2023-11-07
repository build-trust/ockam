import SwiftUI

struct CreateServiceView: View {
    @Environment(\.presentationMode) var presentationMode: Binding<PresentationMode>
    @FocusState private var isFocused: Bool

    @State var isProcessing = false
    @State var errorMessage = ""
    @State var serviceName = ""
    @State var serviceAddress = "localhost:10000"
    @State var emails = Set<String>()

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
                GridRow {
                    VStack(alignment: .leading) {
                        Text(verbatim: "Share")
                        Text(verbatim: "Optionally, share your service with others").font(
                            .caption)
                    }
                    .padding(.top, 6)
                }
            }
            .padding(10)

            EmailListView(emailList: $emails)

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
                        let emails: String

                        emails = Array(self.emails).joined(separator: ";")
                        self.errorMessage = ""

                        isProcessing = true
                        let error = create_local_service(
                            self.serviceName,
                            self.serviceAddress,
                            emails
                        )
                        isProcessing = false

                        if error == nil {
                            self.errorMessage = ""
                            self.serviceName = ""
                            self.emails = Set<String>()
                            self.serviceAddress = "localhost:10000"
                            self.closeWindow()
                        } else {
                            self.errorMessage = String(cString: error.unsafelyUnwrapped)
                        }
                    },
                    label: {
                        Text("Create and Share")
                    }
                )
                .disabled(!canCreateService() && !isProcessing)
                .keyboardShortcut(.defaultAction)
                .padding(10)
            }
            .background(.black.opacity(0.1))
        }
        .frame(width: 600)
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
        CreateServiceView()
    }
}
