let w = window()!
activate(w.pid)
let pivot = CGPoint(x: w.bounds.minX + 369, y: w.bounds.minY + 252)
let start = CGPoint(x: pivot.x + 80, y: pivot.y)
mouse(.mouseMoved, start); usleep(300_000)
shot(w, "/tmp/rw/r0.png")
mouse(.leftMouseDown, start); usleep(120_000)
for i in 1...18 {
    let a = Double(i) * 5.0 * Double.pi / 180.0
    mouse(.leftMouseDragged, CGPoint(x: pivot.x + 80 * cos(a), y: pivot.y + 80 * sin(a))); usleep(30_000)
}
let end = CGPoint(x: pivot.x, y: pivot.y + 80)
mouse(.leftMouseUp, end); usleep(600_000)
shot(w, "/tmp/rw/r1.png")
mouse(.mouseMoved, CGPoint(x: w.bounds.minX + 700, y: w.bounds.minY + 900)); usleep(500_000)
shot(w, "/tmp/rw/r2.png")
