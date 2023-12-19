import SwiftUI

struct EnrollmentStatus: View {
    @Binding var status: OrchestratorStatus

    var body: some View {
        VStack(alignment: .center) {
            switch status {
            case .Disconnected:
                Text("Please enroll to get started")
            case .WaitingForToken:
                Text("Opened account.ockam.io")
                Text("Please finish enrolling in your browser.").font(.caption)
            case .WaitingForEmailValidation:
                Text("We’ve sent you a verification email.\n\nPlease check your inbox and click the included link so we can verify your email address.").padding(.bottom, VerticalSpacingUnit*2)
            case .RetrievingSpace:
                Text("Fetching your spaces...")
            case .RetrievingProject:
                AnimatedEllipsis(text: "Provisioning a dedicated project", interval: 1.0)
                Text("This may take up to 3 minutes.").font(.caption)
            case .Connecting:
                Text("Connecting to Orchestrator")
            case .Connected:
                Text("Encrypted relay is active.")
            }
        }
        .padding(0)
    }
}

struct EnrollmentStatus_Previews: PreviewProvider {
    static var previews: some View {
        VStack(spacing: VerticalSpacingUnit) {
            EnrollmentStatus(status: .constant(.Disconnected))
            EnrollmentStatus(status: .constant(.Connected))
            EnrollmentStatus(status: .constant(.Connecting))
            EnrollmentStatus(status: .constant(.WaitingForToken))
            EnrollmentStatus(status: .constant(.WaitingForEmailValidation))
            EnrollmentStatus(status: .constant(.RetrievingSpace))
            EnrollmentStatus(status: .constant(.RetrievingProject))
        }
        .frame(width: 320, height: 400)
    }
}
