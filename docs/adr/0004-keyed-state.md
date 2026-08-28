# ADR 0004: Keyed transactional state

Status: Accepted

Component state lives in Rust and is identified by structural parent path plus
a stable sibling key. Stateful components require a key. Candidate renders use
transactions; failed renders do not change committed state or clean unreachable
instances.

Schema-compatible fields survive reload. Only incompatible fields reset.
