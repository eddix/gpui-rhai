let ws = windows(owner: "multi_window")
guard let settings = ws.first(where: { $0.bounds.minX >= 0 }) else { exit(1) }
activate(settings.pid)
click(CGPoint(x: settings.bounds.minX + 480, y: settings.bounds.minY + 204)); usleep(700_000)
for (i, w) in windows(owner: "multi_window").enumerated() { shot(w, "/tmp/rw/l\(i).png"); print(i, w.bounds.minX >= 0 ? "settings(visible)" : "main", w.bounds) }
