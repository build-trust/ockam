import SwiftUI

struct AnimatedEllipsis: View {
    let text: String
    let interval: TimeInterval
    @State private var currentIndex = 0

    var body: some View {
        // using a space that matches the same size of '.' to avoid
        // the string to change offset during the animation
        Text(
            text +
            String(repeating: ".", count: currentIndex) +
            String(repeating: "\u{2008}", count: 3 - currentIndex)
        )
        .onAppear {
            Timer.scheduledTimer(withTimeInterval: interval, repeats: true) { _ in
                currentIndex = (currentIndex + 1) % 4
            }
        }
    }
}


struct AnimatedEllipsis_Previews: PreviewProvider {
    static var previews: some View {
        AnimatedEllipsis(text: "Loading", interval: 0.5)
            .padding(20)
    }
}
