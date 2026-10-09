let w = window()!
activate(w.pid)
let start = CGPoint(x: w.bounds.minX + 348, y: w.bounds.minY + 174)
mouse(.mouseMoved, start); usleep(300_000)
shot(w, "/tmp/rw/d0.png")
mouse(.leftMouseDown, start); usleep(120_000)
for i in 1...16 { mouse(.leftMouseDragged, CGPoint(x: start.x + CGFloat(i) * 5, y: start.y + CGFloat(i) * 4)); usleep(30_000) }
mouse(.leftMouseUp, CGPoint(x: start.x + 80, y: start.y + 64)); usleep(600_000)
shot(w, "/tmp/rw/d1.png")
mouse(.mouseMoved, CGPoint(x: w.bounds.minX + 700, y: w.bounds.minY + 950)); usleep(500_000)
shot(w, "/tmp/rw/d2.png")
