import Cocoa
import CoreGraphics

struct Win { let id: Int; let bounds: CGRect; let pid: pid_t }

func window(owner: String = "gpui-rhai", minHeight: CGFloat = 200, index: Int = 0) -> Win? {
    let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as! [[String: Any]]
    var found: [Win] = []
    for info in list {
        guard let name = info[kCGWindowOwnerName as String] as? String, name.contains(owner),
              let b = info[kCGWindowBounds as String] as? [String: CGFloat], (b["Height"] ?? 0) >= minHeight,
              let id = info[kCGWindowNumber as String] as? Int,
              let pid = info[kCGWindowOwnerPID as String] as? pid_t,
              (info[kCGWindowLayer as String] as? Int ?? 0) == 0 else { continue }
        found.append(Win(id: id, bounds: CGRect(x: b["X"]!, y: b["Y"]!, width: b["Width"]!, height: b["Height"]!), pid: pid))
    }
    return index < found.count ? found[index] : nil
}

func windows(owner: String) -> [Win] {
    var all: [Win] = []
    var i = 0
    while let w = window(owner: owner, minHeight: 100, index: i) { all.append(w); i += 1 }
    return all
}

func activate(_ pid: pid_t) {
    NSRunningApplication(processIdentifier: pid)?.activate(options: [.activateIgnoringOtherApps])
    usleep(400_000)
}

func mouse(_ type: CGEventType, _ p: CGPoint, clicks: Int64 = 1) {
    let e = CGEvent(mouseEventSource: nil, mouseType: type, mouseCursorPosition: p, mouseButton: .left)!
    e.setIntegerValueField(.mouseEventClickState, value: clicks)
    e.post(tap: .cghidEventTap)
}

func click(_ p: CGPoint, clicks: Int64 = 1) {
    mouse(.mouseMoved, p); usleep(80_000)
    mouse(.leftMouseDown, p, clicks: clicks); usleep(60_000)
    mouse(.leftMouseUp, p, clicks: clicks); usleep(120_000)
}

func doubleClick(_ p: CGPoint) {
    mouse(.mouseMoved, p); usleep(80_000)
    mouse(.leftMouseDown, p, clicks: 1); usleep(30_000)
    mouse(.leftMouseUp, p, clicks: 1); usleep(60_000)
    mouse(.leftMouseDown, p, clicks: 2); usleep(30_000)
    mouse(.leftMouseUp, p, clicks: 2); usleep(600_000)
}

func key(_ code: CGKeyCode, _ flags: CGEventFlags = []) {
    let down = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: true)!
    down.flags = flags
    down.post(tap: .cghidEventTap)
    usleep(30_000)
    let up = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: false)!
    up.flags = flags
    up.post(tap: .cghidEventTap)
    usleep(150_000)
}

func typeText(_ text: String) {
    for scalar in text.utf16 {
        var unit = scalar
        let down = CGEvent(keyboardEventSource: nil, virtualKey: 0, keyDown: true)!
        down.keyboardSetUnicodeString(stringLength: 1, unicodeString: &unit)
        down.post(tap: .cghidEventTap)
        let up = CGEvent(keyboardEventSource: nil, virtualKey: 0, keyDown: false)!
        up.keyboardSetUnicodeString(stringLength: 1, unicodeString: &unit)
        up.post(tap: .cghidEventTap)
        usleep(40_000)
    }
    usleep(200_000)
}

func shot(_ win: Win, _ path: String) {
    let process = Process()
    process.executableURL = URL(fileURLWithPath: "/usr/sbin/screencapture")
    process.arguments = ["-x", "-o", "-l", String(win.id), path]
    try? process.run()
    process.waitUntilExit()
}

func pasteboard() -> String { NSPasteboard.general.string(forType: .string) ?? "<none>" }

let kTab: CGKeyCode = 48, kEscape: CGKeyCode = 53, kK: CGKeyCode = 40, kA: CGKeyCode = 0,
    kC: CGKeyCode = 8, kV: CGKeyCode = 9, kRight: CGKeyCode = 124, kDown: CGKeyCode = 125,
    kReturn: CGKeyCode = 36, kF10: CGKeyCode = 109
