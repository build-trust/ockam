/*
    Generic component to list emails
*/
import SwiftUI

struct EmailListView: View {
    @State private var emailInput: String = ""
    @Binding public var emailList: Set<String>

    var body: some View {
        VStack(alignment: .leading) {
            HStack {
                TextField("Enter email", text: $emailInput)
                    .onSubmit {
                        if validateEmail(email: self.emailInput) {
                            self.emailList.insert(self.emailInput)
                            self.emailInput = ""
                        }
                    }

                Button(action: {
                    self.emailList.insert(self.emailInput)
                    self.emailInput = ""

                }) {
                    Text("Add").padding([.leading, .trailing], 5)
                }
                .padding([.leading, .trailing], 10)
                .disabled(!validateEmail(email: emailInput))
            }
            List {
                ForEach(Array(emailList), id: \.self) { email in
                    HStack {
                        Text(email)
                        Spacer()
                        Button(action: {
                            self.emailList.remove(email)
                        }) {
                            Image(systemName: "xmark.circle")
                                .imageScale(.large)
                                .foregroundColor(.red)
                        }
                        .buttonStyle(.plain)
                    }
                }
            }
            .frame(height: 100)
        }
        .padding()
    }

    private func validateEmail(email: String) -> Bool {
        // keeping the email regex very loose since unicode is allowed
        // company-specific TLDs are a possibility
        let emailFormat = "[^@]+@[^@]+"
        let emailPredicate = NSPredicate(
            format:"SELF MATCHES %@",
            emailFormat
        )
        return emailPredicate.evaluate(with: email)
    }
}

struct EmailList_Previews: PreviewProvider {
    @State static var emails = Set([
        "one@example.com",
        "two@example.com",
        "three@example.com"
    ])

    static var previews: some View {
        EmailListView(
            emailList: $emails
        )
    }
}
