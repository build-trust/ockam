import SwiftUI

struct EnrollmentStatus: View {
    @Binding var status: OrchestratorStatus

    var body: some View {
        VStack(alignment: .leading) {
            switch status {
            case .Disconnected:
                Text("Please enroll to get started")
            case .Connected:
                Text("Enrolled with Ockam Orchestrator")
            case .Connecting:
                Text("Connecting to Ockam Orchestrator")
            case .WaitingForToken:
                Text("Opened account.ockam.io/activate")
                Text("Waiting for you to authenticate in your browser...").font(.caption)
            case .RetrievingSpace:
                Text("Getting available spaces in your account")
            case .RetrievingProject:
                Text("Getting available projects")
                Text("This might take a few minutes").font(.caption)
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
            EnrollmentStatus(status: .constant(.RetrievingSpace))
            EnrollmentStatus(status: .constant(.RetrievingProject))
        }
        .frame(width: 320, height: 400)
    }
}
