let ws = windows(owner: "multi_window")
print("windows", ws.count)
guard let settings = ws.first(where: { $0.bounds.minX >= 0 }) else { print("none visible"); exit(1) }
activate(settings.pid)
click(CGPoint(x: settings.bounds.minX + 15, y: settings.bounds.minY + 15)); usleep(900_000)
shot(settings, "/tmp/rw/g0.png")
