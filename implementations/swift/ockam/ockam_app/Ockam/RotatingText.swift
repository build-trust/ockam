import SwiftUI

struct RotatingText: View {
    let texts: [String]
    let interval: TimeInterval
    @State private var currentIndex = 0

    var body: some View {
        Text(texts[currentIndex])
            .multilineTextAlignment(.center)
            .onAppear {
                Timer.scheduledTimer(withTimeInterval: interval, repeats: true) { _ in
                    currentIndex = (currentIndex + 1) % texts.count
                }
            }
    }
}
