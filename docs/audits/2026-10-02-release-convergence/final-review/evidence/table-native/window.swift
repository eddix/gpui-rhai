import AppKit
import ApplicationServices
import CoreGraphics
import Foundation

func fail(_ message: String) -> Never {
    FileHandle.standardError.write(Data((message + "\n").utf8))
    exit(1)
}
guard (CommandLine.arguments.count == 4 || CommandLine.arguments.count == 5),
      let pid = Int32(CommandLine.arguments[1]),
      let app = NSRunningApplication(processIdentifier: pid),
      app.bundleIdentifier == CommandLine.arguments[2],
      app.bundleIdentifier?.hasPrefix("com.eddix.gpui-rhai.visual-test.data-table.") == true
else { fail("Expected exact owned test PID and bundle identifier") }
let mode = CommandLine.arguments[3]
guard mode == "place" || mode == "inspect" || mode == "capture" else { fail("Unknown mode") }
guard AXIsProcessTrusted() else { fail("Native window positioning has no Accessibility access") }
let application = AXUIElementCreateApplication(pid)
var value: CFTypeRef?
guard AXUIElementCopyAttributeValue(application, kAXWindowsAttribute as CFString, &value) == .success,
      let windows = value as? [AXUIElement] else { fail("No AX windows") }
func axSize(_ window: AXUIElement) -> CGSize? {
    var value: CFTypeRef?
    guard AXUIElementCopyAttributeValue(window, kAXSizeAttribute as CFString, &value) == .success,
          let value = value, CFGetTypeID(value) == AXValueGetTypeID() else { return nil }
    var size = CGSize.zero
    guard AXValueGetValue(value as! AXValue, .cgSize, &size) else { return nil }
    return size
}
guard let window = windows.first(where: { (axSize($0)?.height ?? 0) > 500 }) else {
    fail("No owned test content window")
}
func place() {
    _ = app.activate(options: [.activateAllWindows])
    _ = AXUIElementPerformAction(window, kAXRaiseAction as CFString)
    var point = CGPoint(x: 100, y: 100)
    var size = CGSize(width: 980, height: 752)
    guard let position = AXValueCreate(.cgPoint, &point),
          let dimensions = AXValueCreate(.cgSize, &size) else { fail("Invalid AX frame values") }
    for _ in 0..<3 {
        guard AXUIElementSetAttributeValue(window, kAXSizeAttribute as CFString, dimensions) == .success,
              AXUIElementSetAttributeValue(window, kAXPositionAttribute as CFString, position) == .success
        else { fail("AX frame update failed") }
        usleep(150_000)
    }
}
func geometry() -> [String: Any]? {
let entries = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID)
    as? [[String: Any]] ?? []
guard let entry = entries.first(where: {
    guard ($0[kCGWindowOwnerPID as String] as? NSNumber)?.int32Value == pid,
          let bounds = $0[kCGWindowBounds as String] as? [String: NSNumber] else { return false }
    return (bounds["Height"]?.doubleValue ?? 0) > 500
}), let bounds = entry[kCGWindowBounds as String] as? [String: NSNumber],
    let number = entry[kCGWindowNumber as String] as? NSNumber,
    let ax = axSize(window) else { return nil }
let result: [String: Any] = [
    "pid": pid, "bundle_id": app.bundleIdentifier!, "window_id": number,
    "frame": bounds, "ax_size": ["width": Double(ax.width), "height": Double(ax.height)],
    "screen_scale": NSScreen.screens.first?.backingScaleFactor ?? 0,
]
return result
}
func exact(_ value: [String: Any]) -> Bool {
    guard let frame = value["frame"] as? [String: NSNumber],
          let ax = value["ax_size"] as? [String: Double],
          let x = frame["X"]?.doubleValue, let y = frame["Y"]?.doubleValue else { return false }
    return frame["Width"]?.doubleValue == 980 && frame["Height"]?.doubleValue == 752
        && ax["width"] == 980 && ax["height"] == 752 && x == x.rounded() && y == y.rounded()
}
var result: [String: Any]
if mode == "capture" {
    guard CommandLine.arguments.count == 5,
          CommandLine.arguments[4].hasPrefix("/tmp/gpui-rhai-018-native-capture.WZYwOr/")
    else { fail("Expected scoped output path") }
    var accepted: [String: Any]?
    for attempt in 1...8 {
        place()
        for _ in 0..<12 {
            usleep(100_000)
            guard let before = geometry(), exact(before),
                  let number = before["window_id"] as? NSNumber else { continue }
            let process = Process()
            process.executableURL = URL(fileURLWithPath: "/usr/sbin/screencapture")
            process.arguments = ["-x", "-o", "-l", number.stringValue, CommandLine.arguments[4]]
            try process.run()
            process.waitUntilExit()
            guard process.terminationStatus == 0,
                  let after = geometry(), exact(after),
                  NSDictionary(dictionary: before).isEqual(to: after)
            else { continue }
            accepted = ["before": before, "after": after, "attempt": attempt,
                        "capture": CommandLine.arguments[4], "capture_tool": "screencapture -x -o -l"]
            break
        }
        if accepted != nil { break }
    }
    guard let accepted = accepted else { fail("Could not capture a stable exact980x752 frame") }
    result = accepted
} else {
    if mode == "place" { place() }
    guard let measured = geometry() else { fail("No on-screen owned content frame") }
    result = measured
}
let data = try JSONSerialization.data(withJSONObject: result, options: [.sortedKeys])
FileHandle.standardOutput.write(data)
FileHandle.standardOutput.write(Data("\n".utf8))
