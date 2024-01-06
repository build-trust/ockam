//
//  PreferencesView.swift
//  Portals, by Ockam
//
//  Created by Adin Ćebić on 6. 1. 2024..
//

import Foundation
import SwiftUI

struct PreferencesView: View {
    @StateObject private var launchServiceObservable = LaunchServiceObservable()

    var body: some View {
        VStack {
            Form {
                Section("Launch options") {
                    Toggle("Automatically start Ockam after login", isOn: $launchServiceObservable.isEnabled)
                }
            }
            Spacer()
        }
        .padding()
    }
}

#Preview {
    PreferencesView()
}
