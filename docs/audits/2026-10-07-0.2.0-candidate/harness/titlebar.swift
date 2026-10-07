guard let w0 = window() else { print("no window"); exit(1) }
activate(w0.pid)
print("before", w0.bounds)
let start = CGPoint(x: w0.bounds.minX + 290, y: w0.bounds.minY + 18)
mouse(.mouseMoved, start); usleep(200_000)
mouse(.leftMouseDown, start); usleep(150_000)
for i in 1...20 { mouse(.leftMouseDragged, CGPoint(x: start.x + CGFloat(i) * 5, y: start.y + CGFloat(i) * 3)); usleep(30_000) }
usleep(250_000)
let during = window()!.bounds
print("during", during, "moved", during.origin != w0.bounds.origin)
mouse(.leftMouseUp, CGPoint(x: start.x + 100, y: start.y + 60)); usleep(800_000)
let after = window()!.bounds
print("after", after)
// Double press on the bar: the platform title-bar action (zoom).
let w1 = window()!
let spot = CGPoint(x: w1.bounds.minX + 290, y: w1.bounds.minY + 18)
doubleClick(spot)
usleep(800_000)
let zoomed = window()!.bounds
print("double press", w1.bounds, "->", zoomed, "changed", zoomed != w1.bounds)
// A press on a bar control stays with the control: the window does not move.
let control = CGPoint(x: w1.bounds.minX + 0, y: 0)
_ = control
