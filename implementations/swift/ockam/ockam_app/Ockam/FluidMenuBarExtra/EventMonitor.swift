//
//  EventMonitor.swift
//  FluidMenuBarExtra
//
//  Created by Lukas Romsicki on 2022-12-17.
//  Copyright © 2022 Lukas Romsicki.
//

import AppKit

class EventMonitor {
    fileprivate let mask: NSEvent.EventTypeMask
    fileprivate var monitor: Any?

    fileprivate init(mask: NSEvent.EventTypeMask) {
        self.mask = mask
    }

    deinit {
        stop()
    }

    func start() {
        fatalError("start must be implemented by a subclass of EventMonitor")
    }

    func stop() {
        if monitor != nil {
            NSEvent.removeMonitor(monitor!)
            monitor = nil
        }
    }
}

final class LocalEventMonitor: EventMonitor {
    typealias Handler = (NSEvent) -> NSEvent?

    private let handler: Handler

    init(mask: NSEvent.EventTypeMask, handler: @escaping Handler) {
        self.handler = handler
        super.init(mask: mask)
    }

    override func start() {
        monitor = NSEvent.addLocalMonitorForEvents(matching: mask, handler: handler)
    }
}

final class GlobalEventMonitor: EventMonitor {
    typealias Handler = (NSEvent) -> Void

    private let handler: Handler

    init(mask: NSEvent.EventTypeMask, handler: @escaping Handler) {
        self.handler = handler
        super.init(mask: mask)
    }

    override func start() {
        monitor = NSEvent.addGlobalMonitorForEvents(matching: mask, handler: handler)
    }
}
