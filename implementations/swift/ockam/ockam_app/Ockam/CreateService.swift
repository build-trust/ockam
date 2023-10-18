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
            Grid(alignment: .leading){
                GridRow {
                    VStack(alignment: .leading) {
                        Text(verbatim: "Service Name")
                        Text(verbatim: "Name of the service you want to share").font(.caption)
                    }
                    TextField("Name", text: $serviceName)
                        .focused($isFocused)
                        .onAppear(perform: {
                            isFocused = true
                        })
                }
                GridRow{
                    VStack(alignment: .leading) {
                        Text(verbatim: "Address")
                        Text(verbatim: "Choose an address for the service").font(.caption)
                    }
                    TextField("Address", text: $serviceAddress)
                }
                GridRow{
                    VStack(alignment: .leading) {
                        Text(verbatim: "Share")
                        Text(verbatim: "Optionally, send an invitation to share this service").font(.caption)
                    }
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
                Button(action: {
                    self.closeWindow()
                }, label: {
                    Text("Close")
                })
                Button(action: {
                    let emails: String;

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
                }, label: {
                    Text("Create and Share")
                })
                .disabled(!canCreateService() && !isProcessing)
                .keyboardShortcut(.defaultAction)
                .padding(10)
            }
            .background(.black.opacity(0.1))
        }
        .frame(width: 600)
    }

    func closeWindow(){
        self.presentationMode.wrappedValue.dismiss()
    }

    func canCreateService() -> Bool{
        return !self.serviceName.isEmpty && !self.serviceAddress.isEmpty
    }
}


struct CreateServiceView_Previews: PreviewProvider {
    static var previews: some View {
        CreateServiceView()
    }
}
