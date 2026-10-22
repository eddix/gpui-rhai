#!/bin/bash
set -eu

# Every reference application under apps/ keeps the registry components, token
# base and palettes it installed in step with the registry of this checkout: a
# pull request that changes one updates the apps in the same change. A copy left
# behind is how disktree kept a pre-#140 SplitPane after upgrading (#156).
#
# The check is the CLI's own update plan, so it also covers fixes that keep a
# component's version. Each app then has to pass `gpui-rhai check`.

cd "$(dirname "$0")/.."

cli=(cargo run --quiet --locked -p gpui-rhai-cli --)
status=0
for manifest in apps/*/.gpui-rhai/manifest.toml; do
  app="${manifest%/.gpui-rhai/manifest.toml}"
  plan="$("${cli[@]}" --root "${app}" --dry-run update)"
  if [[ "${plan}" != "No filesystem changes required." ]]; then
    echo "${app}: installed components differ from the registry:"
    echo "${plan}"
    echo "update them with: cargo run -p gpui-rhai-cli -- --root ${app} update"
    status=1
  fi
  # The app also passes the checks any project gets from `gpui-rhai check`.
  if ! report="$("${cli[@]}" --root "${app}" check 2>&1)"; then
    echo "${app}: gpui-rhai check failed:"
    echo "${report}"
    status=1
  fi
done
exit "${status}"
