import Foundation

import SwiftUI

struct BrokenStateView: View {
    var body: some View {
        VStack {
            Text("The Ockam state is either corrupted or incompatible with the current version.\n\nYou can reset the state or quit the application and try another version.")
                .multilineTextAlignment(.center)
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
