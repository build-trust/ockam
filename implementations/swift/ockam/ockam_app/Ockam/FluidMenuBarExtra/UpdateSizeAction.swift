//
//  UpdateSizeAction.swift
//  FluidMenuBarExtra
//
//  Created by Lukas Romsicki on 2022-12-17.
//  Copyright © 2022 Lukas Romsicki.
//

import SwiftUI

/// Structure representing an action that is called by a child view to notify a parent view
/// that one of its children has resized.
struct UpdateSizeAction {
    typealias Action = (_ size: CGSize) -> Void

    let action: Action

    func callAsFunction(size: CGSize) {
        action(size)
    }
}

private struct UpdateSizeKey: EnvironmentKey {
    static var defaultValue: UpdateSizeAction?
}

extension EnvironmentValues {
    var updateSize: UpdateSizeAction? {
        get { self[UpdateSizeKey.self] }
        set { self[UpdateSizeKey.self] = newValue }
    }
}

extension View {
    /// Adds an action to perform when a child view reports that it has resized.
    /// - Parameter action: The action to perform.
    func onSizeUpdate(_ action: @escaping (_ size: CGSize) -> Void) -> some View {
        let action = UpdateSizeAction { size in
            action(size)
        }

        return environment(\.updateSize, action)
    }
}
