//
//  LaunchService.swift
//  Portals, by Ockam
//
//  Created by Adin Ćebić on 6. 1. 2024..
//

import Foundation

protocol LaunchService {
    var startsOnLogin: Bool { get }
    func registerForStartOnLogin(_ startsOnLogin: Bool)
}
