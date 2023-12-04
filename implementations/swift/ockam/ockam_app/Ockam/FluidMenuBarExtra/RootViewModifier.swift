//
//  RootViewModifier.swift
//  FluidMenuBarExtra
//
//  Created by Lukas Romsicki on 2022-12-16.
//  Copyright © 2022 Lukas Romsicki.
//

import SwiftUI

/// A view modifier that reads the size of its content and posts a notification when
/// the size changes.
///
/// When the parent of the view affected by this modifier updates its size, `RootViewModifier`
/// expands the view to fill the available space, aligning its content to the top. When the window
/// the view is contained in changes scene phase, the current phase is provided through the
/// `scenePhase` environment key.
///
/// When applied, the affected view ignores all safe areas so as to fill the space usually occupied
/// by the title bar.
struct RootViewModifier: ViewModifier {
    @Environment(\.updateSize) private var updateSize

    @State private var scenePhase: ScenePhase = .background

    let windowTitle: String

    func body(content: Content) -> some View {
        content
            .environment(\.scenePhase, scenePhase)
            .edgesIgnoringSafeArea(.all)
            .background(
                GeometryReader { geometry in
                    Color.clear
                        .onAppear {
                            updateSize?(size: geometry.size)
                        }
                        .onChange(of: geometry.size) { newValue in
                            updateSize?(size: geometry.size)
                        }
                }
            )
            .fixedSize()
            .frame(minWidth: 0, maxWidth: .infinity, minHeight: 0, maxHeight: .infinity, alignment: .top)
            .onReceive(NotificationCenter.default.publisher(for: NSWindow.didBecomeKeyNotification)) { notification in
                guard let window = notification.object as? NSWindow, window.title == windowTitle, scenePhase != .active else {
                    return
                }

                scenePhase = .active
            }
            .onReceive(NotificationCenter.default.publisher(for: NSWindow.didResignKeyNotification)) { notification in
                guard let window = notification.object as? NSWindow, window.title == windowTitle, scenePhase != .background else {
                    return
                }

                scenePhase = .background
            }
    }
}
