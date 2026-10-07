let w0 = window()!
activate(w0.pid)
click(CGPoint(x: w0.bounds.minX + 900, y: w0.bounds.minY + 900))
usleep(300_000)
shot(w0, "/tmp/rw/k0.png")
for i in 1...3 { key(kTab); usleep(300_000); shot(w0, "/tmp/rw/k\(i).png") }
key(kK, .maskCommand); usleep(600_000)
shot(w0, "/tmp/rw/k4.png")
typeText("dens")
usleep(400_000)
shot(w0, "/tmp/rw/k5.png")
key(kEscape); usleep(500_000)
shot(w0, "/tmp/rw/k6.png")
