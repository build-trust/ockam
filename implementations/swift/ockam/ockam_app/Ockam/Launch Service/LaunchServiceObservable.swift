//
//  LaunchServiceObservable.swift
//  Portals, by Ockam
//
//  Created by Adin Ćebić on 6. 1. 2024..
//

import Foundation
import Combine

final class LaunchServiceObservable: ObservableObject {
    private let launchService: any LaunchService
    private var cancelBag = Set<AnyCancellable>()
    var isEnabled: Bool {
        get { launchService.startsOnLogin }
        set {
            launchService.registerForStartOnLogin(newValue)
            objectWillChange.send()
        }
    }

    init(launchService: any LaunchService = MainAppLaunchService()) {
        self.launchService = launchService
    }
}
