# imap-proto

Upstream: [`imap-proto`](https://crates.io/crates/imap-proto) 0.10.2, from
[djc/tokio-imap](https://github.com/djc/tokio-imap).
License: MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`). Copyright belongs
to Dirkjan Ochtman and the tokio-imap contributors.

`crate/` is the crates.io release with `patches/` applied in order.

## Patches

### 0001 Gmail FETCH attributes

Parses Gmail's `X-GM-MSGID` and `X-GM-LABELS` FETCH attributes
(`src/parser/gmail.rs`, `AttributeValue::GmailMsgId` and
`AttributeValue::GmailLabels`). Without them, `imap` 2.4 rejects any response
that carries one, so Picacho could not use Gmail's permanent message ids or
show its labels.

### 0002 Unmodeled untagged data

Keeps untagged NAMESPACE, QUOTA, QUOTAROOT, MYRIGHTS, LISTRIGHTS, ACL, ID and
ENABLED data as `Response::Other` text (`resp_other` in
`src/parser/rfc3501.rs`). `imap` 2.4 otherwise takes any untagged response it
can't parse for the server hanging up.

### 0003 Building as a patched dependency

Two mechanical fixes that the registry build hides with `--cap-lints`: three
`src/macros.rs` arms drop a trailing semicolon in expression position, and
`src/lib.rs` allows warnings.

### 0004 Extension data

`src/parser/extensions.rs`:

- FETCH attributes: Gmail's `X-GM-THRID`, OBJECTID's `EMAILID` and
  `THREADID` (RFC 8474), `PREVIEW` (RFC 8970), `SAVEDATE` (RFC 8514), and
  BINARY's `BINARY[…]` and `BINARY.SIZE[…]` (RFC 3516), as new
  `AttributeValue` variants.
- Untagged `ESEARCH`, `SORT`, `THREAD` and `VANISHED (EARLIER)` are kept as
  `Response::Other` text.
- An unsolicited `VANISHED` (QRESYNC, RFC 7162) is reported as
  `Response::Expunge(0)`: `imap` 2.4 passes expunges on as unilateral data,
  where any other unknown response would end the command as if the server
  had hung up. Sequence number 0 never names a message.

### 0005 IMAP4rev2

`capability_data` accepts a CAPABILITY list naming `IMAP4rev2` (RFC 9051)
without `IMAP4rev1`, as servers speaking only rev2 send it; `IMAP4rev2` stays
a `Capability::Atom`. Upstream refuses such a list, so `imap` 2.4 can't read
these servers' capabilities and the session fails.

### 0006 UIDONLY

`message_data_uidfetch` in `src/parser/rfc3501.rs`: once a client enables
UIDONLY (RFC 9586), the server sends `* <uid> UIDFETCH (...)` in place of
FETCH, numbered by UID, whose attributes may leave the UID out. It is
reported as `Response::Fetch(uid, …)` with `AttributeValue::Uid(uid)` added
where missing, so `imap` 2.4's `uid_fetch` and `uid_store` read it as a UID
FETCH reply (and pass it over as unilateral data elsewhere). Without it,
`imap` takes the response for the server hanging up.

Remove these patches when Picacho moves to an `imap`/`imap-proto` release that
parses this data.

## Upgrading

```sh
scripts/vendor.sh rebuild imap-proto <version>
```

Then update the version above and in Picacho's `Cargo.toml`.
