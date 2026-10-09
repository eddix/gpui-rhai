func right(_ p: CGPoint) {
    let move = CGEvent(mouseEventSource: nil, mouseType: .mouseMoved, mouseCursorPosition: p, mouseButton: .right)!
    move.post(tap: .cghidEventTap); usleep(100_000)
    CGEvent(mouseEventSource: nil, mouseType: .rightMouseDown, mouseCursorPosition: p, mouseButton: .right)!.post(tap: .cghidEventTap)
    usleep(60_000)
    CGEvent(mouseEventSource: nil, mouseType: .rightMouseUp, mouseCursorPosition: p, mouseButton: .right)!.post(tap: .cghidEventTap)
    usleep(600_000)
}
let w = window()!
activate(w.pid)
right(CGPoint(x: w.bounds.minX + 450, y: w.bounds.minY + 181))
shot(w, "/tmp/rw/ctx1.png")
key(kEscape); usleep(500_000)
// Keyboard: focus the table with Tab from the page, then Shift+F10.
click(CGPoint(x: w.bounds.minX + 450, y: w.bounds.minY + 213)) // api-02 row: selects it
usleep(400_000)
key(kF10, .maskShift); usleep(700_000)
shot(w, "/tmp/rw/ctx2.png")
key(kEscape); usleep(400_000)
