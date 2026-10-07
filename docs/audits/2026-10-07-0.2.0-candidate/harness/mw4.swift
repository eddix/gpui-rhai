let ws = windows(owner: "multi_window")
guard let settings = ws.first(where: { $0.bounds.minX >= 0 }) else { exit(1) }
activate(settings.pid)
key(kTab); key(kTab); usleep(300_000)
shot(settings, "/tmp/rw/h0.png")
key(kReturn); usleep(1_200_000)
let after = windows(owner: "multi_window")
print("after close:", after.count, after.map { $0.bounds })
if let main = after.first {
    shot(main, "/tmp/rw/h1.png")
}
