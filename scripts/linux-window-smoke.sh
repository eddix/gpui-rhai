#!/bin/bash
set -eu

binary="${1:-target/release/examples/phase0_probe}"
smoke_seconds="${GPUI_RHAI_LINUX_SMOKE_SECONDS:-5}"

if [[ ! -x "${binary}" ]]; then
  echo "Linux window smoke binary is missing or not executable: ${binary}"
  exit 1
fi

smoke_root="$(mktemp -d)"
weston_pid=""
xserver_pid=""
cleanup() {
  if [[ -n "${weston_pid}" ]] && kill -0 "${weston_pid}" 2>/dev/null; then
    kill "${weston_pid}" 2>/dev/null || true
    wait "${weston_pid}" 2>/dev/null || true
  fi
  if [[ -n "${xserver_pid}" ]] && kill -0 "${xserver_pid}" 2>/dev/null; then
    kill "${xserver_pid}" 2>/dev/null || true
    wait "${xserver_pid}" 2>/dev/null || true
  fi
  rm -r -- "${smoke_root}"
}
trap cleanup EXIT

assert_window_stays_alive() {
  backend="$1"
  log="$2"
  shift 2

  status=0
  timeout --signal=TERM "${smoke_seconds}s" "$@" >"${log}" 2>&1 || status=$?
  if [[ "${status}" != "124" && "${status}" != "143" ]]; then
    echo "${backend} window smoke exited before the observation window with status ${status}"
    sed -n '1,200p' "${log}"
    exit 1
  fi
  if grep -Eiq 'panicked at|thread .* panicked|failed to initialize|failed to compile|window should open' "${log}"; then
    echo "${backend} window smoke reported an initialization failure"
    sed -n '1,200p' "${log}"
    exit 1
  fi
  echo "Linux window smoke passed: ${backend}"
}

xserver_log="${smoke_root}/xvfb.log"
Xvfb :99 -screen 0 1280x720x24 -nolisten tcp >"${xserver_log}" 2>&1 &
xserver_pid=$!
for _ in $(seq 1 50); do
  if [[ -S /tmp/.X11-unix/X99 ]]; then
    break
  fi
  if ! kill -0 "${xserver_pid}" 2>/dev/null; then
    echo "Xvfb exited before creating its X11 socket"
    sed -n '1,200p' "${xserver_log}"
    exit 1
  fi
  sleep 0.1
done

if [[ ! -S /tmp/.X11-unix/X99 ]]; then
  echo "Xvfb did not create its X11 socket"
  sed -n '1,200p' "${xserver_log}"
  exit 1
fi

x11_log="${smoke_root}/x11.log"
assert_window_stays_alive \
  X11 \
  "${x11_log}" \
  env -u WAYLAND_DISPLAY DISPLAY=:99 XDG_SESSION_TYPE=x11 "${binary}"

runtime_dir="${smoke_root}/runtime"
mkdir -p "${runtime_dir}"
chmod 700 "${runtime_dir}"
weston_log="${smoke_root}/weston.log"
DISPLAY=:99 XDG_RUNTIME_DIR="${runtime_dir}" \
  weston \
  --backend=x11-backend.so \
  --socket=wayland-gpui-rhai \
  --idle-time=0 \
  --width=1280 \
  --height=720 \
  --log="${weston_log}" &
weston_pid=$!

for _ in $(seq 1 50); do
  if [[ -S "${runtime_dir}/wayland-gpui-rhai" ]]; then
    break
  fi
  if ! kill -0 "${weston_pid}" 2>/dev/null; then
    echo "Weston exited before creating its Wayland socket"
    sed -n '1,200p' "${weston_log}"
    exit 1
  fi
  sleep 0.1
done

if [[ ! -S "${runtime_dir}/wayland-gpui-rhai" ]]; then
  echo "Weston did not create its Wayland socket"
  sed -n '1,200p' "${weston_log}"
  exit 1
fi

wayland_log="${smoke_root}/wayland.log"
assert_window_stays_alive \
  Wayland \
  "${wayland_log}" \
  env -u DISPLAY \
  XDG_RUNTIME_DIR="${runtime_dir}" \
  WAYLAND_DISPLAY=wayland-gpui-rhai \
  XDG_SESSION_TYPE=wayland \
  "${binary}"
