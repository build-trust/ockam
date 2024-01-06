import SwiftUI

struct InviteToPortal: View {
    @Environment(\.presentationMode) var presentationMode: Binding<PresentationMode>
    
    @Binding var state_loaded: Bool
    @State var isProcessing = false
    @State public var localService: LocalService
    @State var emails = Set<String>()
    @State var errorMessage = ""
    
    @Environment(\.colorScheme) var colorScheme
    
    @State private var emailInput: String = ""
    
    var email: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: HorizontalSpacingUnit) {
                TextField(
                    "ex: alice@example.com",
                    text: $emailInput
                )
                .autocorrectionDisabled()
                .padding(.leading, 1)
                .onSubmit {
                    if validateEmail(email: self.emailInput) {
                        self.emails.insert(self.emailInput)
                        self.emailInput = ""
                    }
                }
                
                Button(action: {
                    self.emails.insert(self.emailInput)
                    self.emailInput = ""
                    
                }) {
                    Text("Add").padding([.leading, .trailing], 5)
                }
                .padding([.leading, .trailing], 10)
                .disabled(!validateEmail(email: emailInput))
            }
            .padding(.bottom, 4)
            
            Text("Type an email address and click Add to add it to the invitation list.")
                .font(.caption)
                .foregroundStyle(OckamSecondaryTextColor)
                .padding(.bottom, VerticalSpacingUnit*2)
                .padding(.leading, 4)
            
            ScrollView {
                VStack(spacing: 0) {
                    ForEach(Array(emails), id: \.self) { email in
                        HStack {
                            Text(email)
                            Spacer()
                            Button(action: {
                                self.emails.remove(email)
                            }) {
                                Text("Remove")
                                    .padding([.leading, .trailing], 5)
                                    .foregroundColor(.red)
                                    .underline()
                            }
                            .buttonStyle(.plain)
                        }
                        Spacer()
                    }
                }
            }
            .scrollIndicators(ScrollIndicatorVisibility.never)
            .frame(width: 540, height: 100)
            .padding(VerticalSpacingUnit*2)
            .background( colorScheme == .dark ?
                         Color.black.opacity(0.1) :
                            Color.white.opacity(0.2)
            )
            .overlay(
                RoundedRectangle(cornerRadius: 4)
                    .stroke( colorScheme == .dark ?
                             AnyShapeStyle(Color.white.opacity(0.3)) :
                                AnyShapeStyle(Color.black.opacity(0.2)),
                             lineWidth: 1
                           )
            )
            .cornerRadius(4)
            
            Text("Grant access to the portal: \(localService.name)")
                .font(.caption)
                .foregroundStyle(OckamSecondaryTextColor)
                .padding(.leading, 4)
                .padding(.top, 4)
        }
    }
    
    private func validateEmail(email: String) -> Bool {
        // keeping the email regex very loose since unicode is allowed
        // company-specific TLDs are a possibility
        let emailFormat = "[^@]+@[^@]+"
        let emailPredicate = NSPredicate(
            format: "SELF MATCHES %@",
            emailFormat
        )
        return emailPredicate.evaluate(with: email)
    }
    
    var body: some View {
        VStack(alignment: .center, spacing: 0) {
            
            self.email
                .padding(.top, VerticalSpacingUnit*3)
                .padding(.horizontal, HorizontalSpacingUnit*6)
            
            Hint(
"""
Add a list of email addresses to invite to this Portal.

Once your friends accept their invitation, the '\(localService.name)' service is shared securely over an end-to-end encrypted Portal.

They will have access to it on their localhost.
"""
            )
            
            .padding(.leading, HorizontalSpacingUnit*2)
            .padding(.trailing, HorizontalSpacingUnit*2)
            .padding(.top, VerticalSpacingUnit*2)
            
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
        .frame(width: 600, height: 470)
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
        .frame(height: 470)
    }
}
