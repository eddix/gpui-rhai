## D2: one node key contract

Stacked on #104; retains #97's original author commit (`2b9caeae`) and adds
revision `69a7f1a2`. Independent of Host execution policy. Do not merge or
publish before acceptance of the final combined candidate.

- One canonical ASCII named identifier or one punctuation key, lowercase,
  segment maximum60/full event64; colon, controls, nonASCII, whitespace,
  chords and multiple punctuation characters are rejected.
- Generic on/capture/bubble and on_key_value agree. Ordinary event namespaces
  and NativeHandlerDescriptor remain unchanged.
- Existing key capture/bubble registrations now actually dispatch through
  minimal GPUI capture/on-key routes. Phase order, stop, disabled, Enter/Space
  fallback and Host Escape-owner priority are verified.
- Matching uses GPUI key (including existing logical RTL arrow mapping), not
  key_char, and does not filter modifiers. Exact chords stay with Host actions.
  Ancestor handlers may intentionally consume a key; sibling focus is isolated.

Breaking tightening: on_key_value no longer trims whitespace, and its key
segment maximum is60 rather than64. Use named `space` for the space key.

Validation: unit11, new native8, existing drag3 and keyboard63, strict Clippy,
fmt and diff check pass on this source. IME is GPUI simulated Unicode commit,
not a claim of real OS preedit acceptance. Final platform/manual gates remain
the RC responsibility.
