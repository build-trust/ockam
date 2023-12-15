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
        VStack(alignment: .leading, spacing: 0) {
            Form {
                TextField("Name:", text: $serviceName)
                    .focused($isFocused)
                    .onAppear(perform: {
                        //give focus to the text field on open
                        isFocused = true
                    })
                Text("This is the name your friends will see, ex: My Web App")
                    .font(.caption)
                    .foregroundStyle(OckamSecondaryTextColor)
                    .padding(.bottom, VerticalSpacingUnit)
                    .padding(.leading, 4)

                TextField("Address:", text: $serviceAddress)
                Text("This is the address where your service is listening, ex: localhost:3333 or my-nas:5555")
                    .font(.caption)
                    .foregroundStyle(OckamSecondaryTextColor)
                    .padding(.leading, 4)
            }
            .padding(.top, VerticalSpacingUnit*3)
            .padding(.bottom, VerticalSpacingUnit*2)
            .padding(.horizontal, VerticalSpacingUnit*7)


            Hint(
"""
Pick the TCP or HTTP service you want to share with your friends. After you click 'Open Portal', invite your friends to this Portal from the application menu.
"""
            )
            .padding(.horizontal, HorizontalSpacingUnit*6)

            Spacer()

            HStack {
                if !errorMessage.isEmpty {
                    Text("Error: \(errorMessage)")
                        .foregroundColor(Color(hex: OckamErrorColor))
                        .padding(.leading, HorizontalSpacingUnit*3)
                }
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
        .frame(width: 600, height: 320)
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
