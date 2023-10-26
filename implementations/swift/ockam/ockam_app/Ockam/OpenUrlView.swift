import SwiftUI

struct OpenUrlView: View {
    @Environment(\.presentationMode) var presentationMode: Binding<PresentationMode>
    @Binding var enrolled: Bool

    var body: some View {
        VStack(alignment: .leading) {
            Text("Your invitation is being processed.")
            Text("You can close this window.")
            VStack() {
                HStack {
                    Spacer()
                    Button(
                        action: {
                            self.closeWindow()
                        },
                        label: {
                            Text("Close")
                        })
                }
            }
        }
        .padding(8)
        .frame(width: 400)
        .onOpenURL(perform: { url in
            if let urlComponents = URLComponents(url: url, resolvingAgainstBaseURL: false) {
                // This host matches the `invitations` segment
                var segments = [urlComponents.host]
                // The path contains the `accept` and `invitation_id` segments
                segments.append(
                    contentsOf: urlComponents.path.split(
                        separator: "/", omittingEmptySubsequences: true
                    )
                    .map(String.init))
                if segments.count >= 3 {
                    if segments[0] == "invitations" && segments[1] == "accept" {
                        if enrolled {
                            accept_invitation(segments[2])
                        } else {
                            enroll_user_and_accept_invitation(segments[2])
                        }
                    }
                } else {
                    print("Ignoring URL \(url)")
                }
            }
            self.closeWindow()
        })
    }

    func closeWindow() {
        self.presentationMode.wrappedValue.dismiss()
    }
}

struct OpenUrlView_Previews: PreviewProvider {
    static var previews: some View {
        OpenUrlView(enrolled: .constant(true))
    }
}
