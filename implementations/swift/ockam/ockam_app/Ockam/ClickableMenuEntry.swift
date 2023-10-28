import SwiftUI

// Reproduction of the menu entry since it's not possible
// to inherit a button with the style in macOS 13
struct ClickableMenuEntry: View {
    @State private var isHovered = false

    @State var text: String
    @State var clicked: String = ""
    @State var icon: String = ""
    @State var action: (() -> Void)? = nil
    @State var isDown = false

    var body: some View {
        HStack {
            if icon != "" {
                Image(systemName: icon)
                    .frame(minWidth: 16, maxWidth: 16)
            }
            Text(verbatim: isDown ? (clicked.isEmpty ? text : clicked) : text)
            Spacer()
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 4)
        .background(isHovered ? Color.gray.opacity(0.25) : Color.clear)
        .buttonStyle(PlainButtonStyle())
        .cornerRadius(4)
        .contentShape(Rectangle())
        .modifier(
            PressActions(
                onPress: {
                    isDown = true
                },
                onRelease: {
                    if isDown {
                        isDown = false
                        if let action = action {
                            action()
                        }
                    }
                }
            )
        )
        .onHover { hover in
            isHovered = hover
        }
    }
}
