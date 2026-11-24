# Local GPUI rendering profile

This directory contains the forked packages used by the opt-in macOS
`vendor-gpui` profile. The regular workspace uses crates.io packages.
The root patch configuration selects the same Rust package identities
throughout a sandboxed build; it does not link two GPUI providers.

| Package | Original crate release | SHA-256 of original archive |
| --- | --- | --- |
| gpui-pre | 0.3.11 | `9303c771abf2acff2a96d706f69d9a82088c4770173a07cce88f0002431d887d` |
| gpui-pre-apple | 0.3.11 | `8c0bea8ffa563d3a1ada533e148bb55be7fb873c178979e889a3d1204f8e1b91` |
| gpui-pre-macros | 0.3.11 | `ee1c52959baa6834db3183e86affe70242ca90b4fc64ed2ae6dd689a799ce78e` |
| gpui-fast | 0.1.13 | `7ee67914f779fdcfce4c7f59f6ba8fa68f680bd137648b25a801cd5d9cb1b8ab` |
| gpui-fast-apple | 0.1.13 | `6cdf402e4a795abb27cd9545a6b519f123f4f2b8050fc7f6e7e25e376f34867d` |
| gpui-fast-macros | 0.1.13 | `46724abd05d2c874cc31e1996bb862539c5735780dd363cb550469003a6e5140` |
| zed-font-kit (`font-kit/`) | 0.14.1-zed | `a3898e450f36f852edda72e3f985c34426042c4951790b23b107f93394f9bff5` |

The GPUI snapshots/fork are Apache-2.0. Font-kit is MIT or Apache-2.0.
Licenses and upstream notices are preserved in their corresponding
package directories.

Changes cover layered fills, the Metal scene compositor and atlas,
shaped tracking and text wrapping, intrinsic transitions, and consistent
visual/event coordinates for interactive transforms. Fast's retained
paint and layout paths also carry these changes. Revalidate the actual
renderer and the whole native package family before rebasing. These
packages are not separately published by this repository.
