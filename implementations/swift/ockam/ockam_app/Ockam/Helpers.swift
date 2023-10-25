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
