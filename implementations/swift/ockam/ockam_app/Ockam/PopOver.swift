import SwiftUI


struct PopOver: View {
    @Environment(\.presentationMode) var presentationMode: Binding<PresentationMode>
    @EnvironmentObject private var appDelegate: AppDelegate

    @Binding var state: ApplicationState

    @State private var showIntro: Bool
    //move back the focus to the popover after the browser interaction is complete
    @State private var showPopOnEnrollment: Bool = false

    var body: some View {
        if showIntro {
            GuidedIntro(
                status: $state.orchestrator_status,
                onEnroll: {
                    self.showPopOnEnrollment = true
                },
                onFinish: {
                    self.showIntro = false
                    bringInFront()
                }
            )
            .onReceive(state.$orchestrator_status, perform: { newValue in
                if newValue != .WaitingForToken && newValue != .Disconnected {
                    if self.showPopOnEnrollment  {
                        appDelegate.showPopover()
                    }
                    self.showPopOnEnrollment = false
                }
            })
        } else {
            MainView(state: $state)
        }
    }

    init(state: Binding<ApplicationState>) {
        self._state = state
        self._showIntro = State(initialValue: !state.wrappedValue.enrolled)
    }

    func closeWindow() {
        self.presentationMode.wrappedValue.dismiss()
    }
}

struct PopOver_Previews: PreviewProvider {
    @State static var state = swift_demo_application_state()

    static var previews: some View {
        PopOver(state: $state)
    }
}
