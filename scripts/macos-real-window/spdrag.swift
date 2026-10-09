let w = window()!
activate(w.pid)
// The first SplitPane's separator: 520px wide, size 0.35, starting at the page content edge.
let start = CGPoint(x: w.bounds.minX + 268 + 182 + 4, y: w.bounds.minY + 220)
mouse(.mouseMoved, start); usleep(300_000)
shot(w, "/tmp/rw/s0.png")
mouse(.leftMouseDown, start); usleep(120_000)
for i in 1...16 { mouse(.leftMouseDragged, CGPoint(x: start.x + CGFloat(i) * 5, y: start.y)); usleep(30_000) }
mouse(.leftMouseUp, CGPoint(x: start.x + 80, y: start.y)); usleep(600_000)
shot(w, "/tmp/rw/s1.png")
mouse(.mouseMoved, CGPoint(x: w.bounds.minX + 700, y: w.bounds.minY + 950)); usleep(500_000)
shot(w, "/tmp/rw/s2.png")
