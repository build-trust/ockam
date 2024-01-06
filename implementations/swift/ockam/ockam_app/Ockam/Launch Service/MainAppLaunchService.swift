//
//  MainAppLaunchService.swift
//  Portals, by Ockam
//
//  Created by Adin Ćebić on 6. 1. 2024..
//

import Foundation
import ServiceManagement

final class MainAppLaunchService: LaunchService {
    private let service: SMAppService
    var startsOnLogin: Bool {
        service.status == .enabled
    }

    init() {
        self.service = .mainApp
    }

    func registerForStartOnLogin(_ startsOnLogin: Bool) {
        if startsOnLogin {
            registerForStartOnLogin()
        } else {
            unregisterForStartOnLogin()
        }
    }

    private func registerForStartOnLogin() {
        do {
            try service.register()
        } catch {
            print(error)
        }
    }

    private func unregisterForStartOnLogin() {
        do {
            try service.unregister()
        } catch {
            print(error)
        }
    }
}
