let w0 = window()!
activate(w0.pid)
key(kK, .maskCommand); usleep(700_000)
shot(w0, "/tmp/rw/t0.png")
for code: CGKeyCode in [2, 14, 45, 1] { key(code) }
usleep(500_000)
shot(w0, "/tmp/rw/t1.png")
key(kEscape); usleep(500_000)
