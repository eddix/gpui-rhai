let w0 = window()!
activate(w0.pid)
let compact = CGPoint(x: w0.bounds.minX + 817, y: w0.bounds.minY + 18)
shot(w0, "/tmp/rw/c0.png")
mouse(.mouseMoved, compact); usleep(150_000)
mouse(.leftMouseDown, compact); usleep(120_000)
for i in 1...10 { mouse(.leftMouseDragged, CGPoint(x: compact.x + CGFloat(i) * 4, y: compact.y + CGFloat(i) * 2)); usleep(30_000) }
usleep(200_000)
let during = window()!.bounds
print("control press: before", w0.bounds.origin, "during", during.origin, "moved", during.origin != w0.bounds.origin)
mouse(.leftMouseUp, CGPoint(x: compact.x + 40, y: compact.y + 20)); usleep(500_000)
// A plain click on the segment switches density.
click(compact)
usleep(500_000)
shot(window()!, "/tmp/rw/c1.png")
let comfortable = CGPoint(x: w0.bounds.minX + 607, y: w0.bounds.minY + 18)
click(comfortable)
usleep(400_000)
