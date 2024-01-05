//
//  FluidMenuBarExtraStatusItem.swift
//  FluidMenuBarExtra
//
//  Created by Lukas Romsicki on 2022-12-17.
//  Copyright © 2022 Lukas Romsicki.
//  Copyright © 2023 Ockam.
//

import AppKit
import SwiftUI

/// An individual element displayed in the system menu bar that displays a window
/// when triggered.
final class FluidMenuBarExtraStatusItem: NSObject, NSWindowDelegate {
    private let window: NSWindow
    private let statusItem: NSStatusItem

    private var localEventMonitor: EventMonitor?
    private var globalEventMonitor: EventMonitor?

    private init(window: NSWindow) {
        self.window = window

        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        statusItem.isVisible = true

        super.init()

        statusItem.button?.target = self
        statusItem.button?.action = #selector(didPressStatusBarButton(_:))

        localEventMonitor = LocalEventMonitor(mask: [.keyDown], handler: { [weak self] event in
            let escapeKeyCode = 53
            if event.keyCode == escapeKeyCode {
                self?.dismissWindow()
            }
            return event
        })

        globalEventMonitor = GlobalEventMonitor(mask: [.leftMouseDown, .rightMouseDown]) { [weak self] event in
            if let window = self?.window, window.isKeyWindow {
                // Resign key window status if a external non-activating event is triggered,
                // such as other system status bar menus.
                window.resignKey()
            }
        }

        window.delegate = self
        localEventMonitor?.start()
    }

    deinit {
        NSStatusBar.system.removeStatusItem(statusItem)
    }

    @objc
    private func didPressStatusBarButton(_ sender: NSStatusBarButton) {
        if window.isVisible {
            dismissWindow()
            return
        }

        setWindowPosition()

        // This needs to be called on main queue to keep button highlighting.
        DispatchQueue.main.async {
            sender.highlight(true)
        }

        // Tells the system to persist the menu bar in full screen mode.
        DistributedNotificationCenter.default().post(name: .beginMenuTracking, object: nil)
        window.makeKeyAndOrderFront(nil)
    }

    func windowDidBecomeKey(_ notification: AppKit.Notification) {
        globalEventMonitor?.start()
        setButtonHighlighted(to: true)
    }

    func windowDidResignKey(_ notification: AppKit.Notification) {
        globalEventMonitor?.stop()
        dismissWindow()
    }

    public func showWindow() {
        if window.isVisible {
            return
        }

        setWindowPosition()
        // Tells the system to persist the menu bar in full screen mode.
        DistributedNotificationCenter.default().post(name: .beginMenuTracking, object: nil)
        window.makeKeyAndOrderFront(nil)
    }

    public func dismissWindow() {
        // Tells the system to cancel persisting the menu bar in full screen mode.
        DistributedNotificationCenter.default().post(name: .endMenuTracking, object: nil)

        NSAnimationContext.runAnimationGroup { context in
            context.duration = 0.3
            context.timingFunction = CAMediaTimingFunction(name: .easeInEaseOut)

            window.animator().alphaValue = 0
        }

        // Instead of using animation completion, which could be unreliable,
        // use an async operation with the same animation duration
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) { [weak self] in
            self?.window.orderOut(nil)
            self?.window.alphaValue = 1
            self?.setButtonHighlighted(to: false)
        }
    }

    private func setButtonHighlighted(to highlight: Bool) {
        statusItem.button?.highlight(highlight)
    }

    private func setWindowPosition() {
        guard let statusItemWindow = statusItem.button?.window else {
            // If we don't know where the status item is, just place the window in the center.
            window.center()
            return
        }

        var targetRect = statusItemWindow.frame

        if let screen = statusItemWindow.screen {
            let windowWidth = window.frame.width

            if statusItemWindow.frame.origin.x + windowWidth > screen.visibleFrame.width {
                targetRect.origin.x += statusItemWindow.frame.width
                targetRect.origin.x -= windowWidth

                // Offset by window border size to align with highlighted button.
                targetRect.origin.x += Metrics.windowBorderSize

            } else {
                // Offset by window border size to align with highlighted button.
                targetRect.origin.x -= Metrics.windowBorderSize
            }
        } else {
            // If there's no screen, assume default positioning.
            targetRect.origin.x -= Metrics.windowBorderSize
        }

        window.setFrameTopLeftPoint(targetRect.origin)
    }
}

extension FluidMenuBarExtraStatusItem {
    convenience init(title: String, window: NSWindow) {
        self.init(window: window)

        statusItem.button?.title = title
        statusItem.button?.setAccessibilityTitle(title)
    }

    convenience init(title: String, image: String, window: NSWindow) {
        self.init(window: window)

        statusItem.button?.setAccessibilityTitle(title)
        statusItem.button?.image = NSImage(named: image)
        statusItem.button?.image?.isTemplate = true
    }

    convenience init(title: String, systemImage: String, window: NSWindow) {
        self.init(window: window)

        statusItem.button?.setAccessibilityTitle(title)
        statusItem.button?.image = NSImage(systemSymbolName: systemImage, accessibilityDescription: title)
    }
}

private extension AppKit.Notification.Name {
    static let beginMenuTracking = AppKit.Notification.Name("com.apple.HIToolbox.beginMenuTrackingNotification")
    static let endMenuTracking = AppKit.Notification.Name("com.apple.HIToolbox.endMenuTrackingNotification")
}

private enum Metrics {
    static let windowBorderSize: CGFloat = 2
}
