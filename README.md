# Picacho Patches

Third-party crates that [Picacho](https://github.com/pinaltech/Picacho) uses
with small patches, pulled in through Cargo's `[patch.crates-io]` (gpui-mobile,
which crates.io doesn't carry, as a Git dependency).

| Software | Patches |
| --- | --- |
| [blitz-dom](blitz-dom) 0.3.0-beta.4 | Unresolvable URLs don't panic; tables restacked for narrow screens; legacy table attributes; text selection hit where it is painted |
| [blitz-paint](blitz-paint) 0.3.0-beta.4 | Inline gradient backgrounds |
| [fcm_receiver_rs](fcm_receiver_rs) `fc43098` | Android messaging tokens use the Firebase installation ID and auth token; received messages are acknowledged, so they aren't delivered again |
| [gpui-component](gpui-component) 0.9.1 | Window resize edges over the content |
| [gpui-mobile](gpui-mobile) `5fd54f4` | Builds against gpui-pre 0.3.12 (Android and iOS) |
| [imap-proto](imap-proto) 0.10.2 | Gmail FETCH attributes; unmodeled untagged data; builds as a patched dependency; extension data (thread/email ids, PREVIEW, SAVEDATE, BINARY, ESEARCH, SORT, THREAD, VANISHED); IMAP4rev2-only servers; UIDONLY's UIDFETCH |
| [stylo_derive](stylo_derive) 0.22.0 | ToCss explicit error type |

Each folder has the crate (`crate/`), its patches, its license and a README.
A folder with an `UPSTREAM` file takes its crate from a Git commit rather
than a crates.io release.

All rights to this software belong to its original authors. Each crate keeps
its upstream license, found in its folder; the patches are offered under the
same license.
