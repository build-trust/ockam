import SwiftUI

// Reproduction of the menu entry since it's not possible
// to inherit a button with the style in macOS 13
struct ClickableMenuEntry: View {
    @State var text: String
    @State var clicked: String = ""
    @State var icon: String = ""
    @State var shortcut: String = ""
    @State var action: (() -> Void)? = nil
    @State var textPadding = 0.0
    @State var compact = true

    @State private var isHovered = false
    @State private var isDown = false

    var body: some View {
        HStack(spacing: 0) {
            if icon != "" {
                Image(systemName: icon)
                    .frame(width: 20)
                    .font(.system(size: 12, weight: .bold))
                    .padding(.trailing, StandardIconTextSpacing)

                Text(
                    verbatim: isDown ? (
                        clicked.isEmpty ? text : clicked
                    ) : text
                )
            } else {
                Text(
                    verbatim: isDown ? (
                        clicked.isEmpty ? text : clicked
                    ) : text
                )
                .padding(.leading, textPadding)
            }


            Spacer()

            if shortcut != "" {
                Text(shortcut)
                    .foregroundColor(Color.gray.opacity(0.6))
            }
        }
        .padding(.horizontal, HorizontalSpacingUnit)
        .frame(height: compact ? VerticalSpacingUnit*3.5 : VerticalSpacingUnit*4)
        .background(
            isHovered ? Color.gray.opacity(0.25) : Color.clear
        )
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
        .accessibilityElement(children: .combine)
        .accessibilityAddTraits(.isButton)
    }
}

struct ClickableMenuEntry_Previews: PreviewProvider {
    static var previews: some View {
        VStack(spacing: 0) {
            ClickableMenuEntry(text: "Hello")
            ClickableMenuEntry(text: "World!", shortcut: "⌘W")
            ClickableMenuEntry(
                text: "Click and hold me!",
                clicked: "Ough!",
                icon: "heart.fill"
            )
        }
        .frame(width: 320, height: 200)
    }
}
