#!/bin/bash
set -eu

settings_cases=(
  catppuccin-latte.en.ltr
  catppuccin-mocha.ar.rtl
  catppuccin-mocha.en.ltr
  default-dark.en.ltr
  default-dark.zh-cn.button-focus
  default-dark.zh-cn.switch-focus
  default-light.en.ltr
  tokyo-night.en.ltr
  tokyo-storm.en.ltr
)
dashboard_cases=(
  catppuccin-latte.en.ltr.normal
  catppuccin-latte.en.ltr.reduced
  catppuccin-mocha.ar.rtl.normal
  catppuccin-mocha.ar.rtl.reduced
  catppuccin-mocha.en.ltr.normal
  catppuccin-mocha.en.ltr.reduced
  default-dark.en.ltr.normal
  default-dark.en.ltr.reduced
  default-light.en.ltr.normal
  default-light.en.ltr.reduced
  tokyo-night.en.ltr.normal
  tokyo-night.en.ltr.reduced
  tokyo-storm.en.ltr.normal
  tokyo-storm.en.ltr.reduced
)
form_cases=(
  catppuccin-latte.en.ltr
  catppuccin-mocha.ar.rtl
  catppuccin-mocha.en.ltr
  default-dark.en.ltr
  default-light.en.dialog
  default-light.en.ltr
  default-light.en.toast
  tokyo-night.en.ltr
  tokyo-storm.en.ltr
)
embedded_view_cases=(
  default-dark.shared-host
)
data_table_cases=(
  catppuccin-mocha.ar.rtl
  default-dark.en.selected
  default-light.en.empty
  default-light.en.ltr
  default-light.en.loading
)

check_case() {
  file="$1"
  expected_width="$2"
  expected_height="$3"
  if [[ ! -s "${file}" ]]; then
    echo "missing or empty visual baseline: ${file}" >&2
    exit 1
  fi
  dimensions="$(python3 - "${file}" <<'PY'
import struct
import sys
import zlib

data = open(sys.argv[1], "rb").read()
if len(data) < 33 or data[:8] != b"\x89PNG\r\n\x1a\n":
    raise SystemExit("not a valid PNG header")
offset = 8
idat = bytearray()
width = height = bit_depth = color_type = interlace = None
ended = False
while offset + 12 <= len(data):
    length = struct.unpack(">I", data[offset:offset + 4])[0]
    kind = data[offset + 4:offset + 8]
    start = offset + 8
    end = start + length
    if end + 4 > len(data):
        raise SystemExit("truncated PNG chunk")
    payload = data[start:end]
    expected_crc = struct.unpack(">I", data[end:end + 4])[0]
    if zlib.crc32(kind + payload) & 0xffffffff != expected_crc:
        raise SystemExit("invalid PNG chunk CRC")
    if kind == b"IHDR":
        width, height, bit_depth, color_type, _, _, interlace = struct.unpack(">IIBBBBB", payload)
    elif kind == b"IDAT":
        idat.extend(payload)
    elif kind == b"IEND":
        ended = True
        offset = end + 4
        break
    offset = end + 4
if not ended or offset != len(data) or width is None or not idat:
    raise SystemExit("incomplete PNG")
decoded = zlib.decompress(idat)
if interlace == 0:
    channels = {0: 1, 2: 3, 3: 1, 4: 2, 6: 4}.get(color_type)
    if channels is None:
        raise SystemExit("unsupported PNG color type")
    row_bytes = (width * channels * bit_depth + 7) // 8
    if len(decoded) != (row_bytes + 1) * height:
        raise SystemExit("unexpected decoded PNG size")
print(f"{width}x{height}")
PY
)"
  if [[ "${dimensions}" != "${expected_width}x${expected_height}" ]]; then
    echo "unexpected visual baseline size: ${file} is ${dimensions}" >&2
    exit 1
  fi
}

for case_name in "${settings_cases[@]}"; do
  check_case "tests/visual/macos/settings_panel/${case_name}.png" 640 552
done
for case_name in "${dashboard_cases[@]}"; do
  check_case "tests/visual/macos/dashboard_layout/${case_name}.png" 760 592
done
for case_name in "${form_cases[@]}"; do
  check_case "tests/visual/macos/form_showcase/${case_name}.png" 760 752
done
for case_name in "${data_table_cases[@]}"; do
  check_case "tests/visual/macos/data_table/${case_name}.png" 980 752
done
for case_name in "${embedded_view_cases[@]}"; do
  check_case "tests/visual/macos/embedded_views/${case_name}.png" 900 452
done

actual_count="$(find tests/visual/macos -type f -name '*.png' | wc -l | tr -d ' ')"
if [[ "${actual_count}" != "38" ]]; then
  echo "unexpected visual baseline count: ${actual_count} (expected 38)" >&2
  exit 1
fi

echo "visual baseline audit passed: ${actual_count} PNG files"
