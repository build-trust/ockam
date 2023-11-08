import Foundation

import SwiftUI

struct BrokenStateView: View {
    var body: some View {
        VStack {
            Text("The local state of Ockam on this machine might have been corrupted or is incompatible with the current version.\n\nPlease reset the state and enroll again.")
                .multilineTextAlignment(.leading)
                .padding()
            Spacer()
            HStack {
                Spacer()
                Button(
                    action: {
                        restartCurrentProcess()
                    },
                    label: {
                        Text("Reset State")
                    }
                    )
                Button(
                    action: {
                        exit(1)
                    },
                    label: {
                        Text("Quit")
                    }
                )
                .keyboardShortcut(.defaultAction)
                .padding(10)
            }
            .background(.black.opacity(0.1))
        }
        .frame(width: 300, height: 160)
    }

}

struct BrokenState_Previews: PreviewProvider {
    static var previews: some View {
        BrokenStateView()
    }
}
