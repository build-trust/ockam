import SwiftUI

// Helper modifier to intercept the 'mouse down' event
struct PressActions: ViewModifier {
    var onPress: () -> Void
    var onRelease: () -> Void
    func body(content: Content) -> some View {
        content
            .simultaneousGesture(
                DragGesture(minimumDistance: 0)
                    .onChanged({ _ in
                        onPress()
                    })
                    .onEnded({ _ in
                        onRelease()
                    })
            )
    }
}

// Helper function to move the application windows to the front
func bringInFront() {
    NSApplication.shared.activate(ignoringOtherApps: true)
}

// Helper function to copy the text into the clipboard
func copyToClipboard(_ text: String) {
    let pasteboard = NSPasteboard.general
    pasteboard.declareTypes([.string], owner: nil)
    pasteboard.setString(text, forType: .string)
}

// this functions reset the state in the file system and
// stops the application node and restart the whole process
func restartCurrentProcess() {
    // first reset the file system
    reset_application_state()

    // start a brand new process, in this phase
    // the application node is already stopped and
    // the local status is cleaned
    let task = Process()
    task.launchPath = Bundle.main.executablePath!
    task.arguments = CommandLine.arguments
    task.launch()

    // and quits
    exit(0)
}


func parseInvitationIdFromUrl(url: URL) -> String? {
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
                return segments[2].unsafelyUnwrapped
            }
        } else {
            print("Ignoring URL \(url)")
        }
    }

    return nil
}

func isRunningPreviewMode() -> Bool {
    return ProcessInfo.processInfo.environment["XCODE_RUNNING_FOR_PREVIEWS"] == "1"
}

extension Color {
    init(hex: Int, opacity: Double = 1.0) {
        let red = Double((hex & 0xff0000) >> 16) / 255.0
        let green = Double((hex & 0xff00) >> 8) / 255.0
        let blue = Double((hex & 0xff) >> 0) / 255.0
        self.init(.sRGB, red: red, green: green, blue: blue, opacity: opacity)
    }
}
