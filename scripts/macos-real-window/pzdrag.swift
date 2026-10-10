let w = window()!
activate(w.pid)
let start = CGPoint(x: w.bounds.minX + 508, y: w.bounds.minY + 290)
mouse(.mouseMoved, start); usleep(300_000)
shot(w, "/tmp/rw/p0.png")
mouse(.leftMouseDown, start); usleep(120_000)
for i in 1...15 { mouse(.leftMouseDragged, CGPoint(x: start.x + CGFloat(i) * 4, y: start.y + CGFloat(i) * 2)); usleep(30_000) }
let end = CGPoint(x: start.x + 60, y: start.y + 30)
mouse(.leftMouseUp, end); usleep(600_000)
// No further input: what the window shows on its own.
shot(w, "/tmp/rw/p1.png")
// Leave the canvas: any redraw the release needed has happened by now.
mouse(.mouseMoved, CGPoint(x: w.bounds.minX + 700, y: w.bounds.minY + 900)); usleep(500_000)
shot(w, "/tmp/rw/p2.png")
