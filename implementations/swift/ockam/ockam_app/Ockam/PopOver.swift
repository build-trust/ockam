import SwiftUI


struct PopOver: View {
    @Environment(\.presentationMode) var presentationMode: Binding<PresentationMode>
    @Binding var state: ApplicationState
    @State private var showIntro: Bool

    var body: some View {
        if showIntro {
            GuidedIntro(
                status: $state.orchestrator_status,
                onFinish: {
                    self.showIntro = false
                    bringInFront()
                }
            )
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
