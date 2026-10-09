func both(_ tag: String) {
    let ws = windows(owner: "multi_window")
    print(tag, "windows:", ws.count)
    for (i, w) in ws.enumerated() { shot(w, "/tmp/rw/mw-\(tag)-\(i).png") }
}
both("start")
let ws = windows(owner: "multi_window")
// The visible one is the settings window (on screen).
guard let settings = ws.first(where: { $0.bounds.minX >= 0 }) else { print("no visible window"); exit(1) }
activate(settings.pid)
click(CGPoint(x: settings.bounds.minX + 129, y: settings.bounds.minY + 204)); usleep(400_000)
click(CGPoint(x: settings.bounds.minX + 340, y: settings.bounds.minY + 204)); usleep(400_000)
both("edited")
// Close through the window's close button: the close handler asks first.
click(CGPoint(x: settings.bounds.minX + 15, y: settings.bounds.minY + 15)); usleep(700_000)
both("asked")
