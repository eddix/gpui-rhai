guard let main = windows(owner: "multi_window").first else { exit(1) }
activate(main.pid)
click(CGPoint(x: main.bounds.minX + 868, y: main.bounds.minY + 204)); usleep(1_200_000)
let opened = windows(owner: "multi_window")
print("after open:", opened.count)
// Light theme in main only.
let m = windows(owner: "multi_window").first(where: { $0.id == main.id }) ?? main
activate(m.pid)
click(CGPoint(x: m.bounds.minX + 480, y: m.bounds.minY + 204)); usleep(600_000)
for (i, w) in windows(owner: "multi_window").enumerated() { shot(w, "/tmp/rw/j\(i).png"); print(i, w.id == main.id ? "main" : "settings", w.bounds) }
