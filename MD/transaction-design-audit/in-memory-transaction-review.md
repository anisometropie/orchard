# In-memory transaction review

Reviewer: `in_memory_tx_review`

Verdict: the happy-path design is sound, but it is not yet absolutely sound.
The review found three high-severity contract holes.

## High severity

### 1. A panic leaves `InMemoryOrchardStorage` permanently inside a zombie transaction

`api/src/adapters/secondary/in_memory_orchard_storage/mod.rs:163` installs the
transaction, calls the closure, then only clears it at line 165. Unwinding
skips `take()`.

Consequences after `catch_unwind`:

- abandoned staged records remain;
- `is_legacy_tree_already_imported()` sees them;
- every later `transaction()` reports `AtomicOperationCouldNotBegin`;
- there is no recovery path.

Cleanup must be guaranteed on unwind, then the panic resumed.

### 2. Writes outside a transaction have no consistent contract

The trait publicly exposes writes independently of `transaction()` at
`api/src/hexagon/ports/orchard_storage.rs:25`.

In-memory behavior is input-dependent:

- `find_or_create_plant_identity()` succeeds outside a transaction when the
  identity already exists because it returns before checking transaction state
  at `api/src/adapters/secondary/in_memory_orchard_storage/mod.rs:221`.
- A new identity fails at line 232.
- `save_tree()` fails only after performing other validations at line 267.

PostgreSQL instead silently autocommits those same calls at
`api/src/adapters/secondary/postgres_orchard_storage/mod.rs:57`.

Choose one rule and enforce it identically. Given the current use cases,
rejecting writes outside an active transaction is safest.

### 3. Nested transaction behavior differs dangerously

In-memory rejects nesting at
`api/src/adapters/secondary/in_memory_orchard_storage/mod.rs:160`. PostgreSQL
has no active-transaction state and sends another `BEGIN` at
`api/src/adapters/secondary/postgres_orchard_storage/mod.rs:30`.

PostgreSQL normally warns and keeps the existing transaction; the inner
`COMMIT` can then commit the outer work prematurely. An outer error can no
longer roll it back. PostgreSQL must reject nesting before issuing SQL.

## Medium severity

### 4. `trees()` lacks read-your-writes parity

In-memory reads only committed vectors at
`api/src/adapters/secondary/in_memory_orchard_storage/mod.rs:283`. PostgreSQL,
using the active connection, sees its uncommitted inserts.

Either combine committed and staged records in-memory or explicitly forbid
`trees()` inside transactions in both adapters.

### 5. Caught storage errors produce different outcomes

In-memory does not mark a transaction failed. A closure can catch a duplicate
`save_tree()` error and return `Ok`, causing earlier staged changes to commit.

PostgreSQL constraint errors abort the transaction; its eventual `COMMIT`
fails or rolls everything back. Define storage-operation errors as terminal for
the transaction, or introduce savepoint behavior consistently.

### 6. The in-memory identity model assumes contiguous positional IDs

IDs are calculated from vector positions/counts at
`api/src/adapters/secondary/in_memory_orchard_storage/mod.rs:221`, validated by
count at line 271, and resolved with `id - 1` at line 292.

PostgreSQL identity sequences develop gaps after rollbacks and conflict
attempts. IDs should be stored explicitly with identities and treated as
opaque, not inferred from vector positions.

## What is correct

- The private `InMemoryOrchardTransaction` struct is the right shape.
- Staged identity/tree writes are invisible to observers.
- Duplicate legacy checks correctly inspect committed and staged trees.
- Ordinary closure errors discard staging.
- Simulated commit failure discards staging.
- Successful publication of identities and trees happens under one mutex, so
  observers cannot see a half-committed orchard.
- With the current single writer and HTTP mutex, observer concurrency is safe.

## Missing tests

A shared adapter contract suite should exercise both implementations:

- panic rollback and subsequent storage reuse;
- nested transaction rejection without premature persistence;
- write calls outside transactions;
- `trees()` visibility inside a transaction;
- caught duplicate-write error semantics;
- reuse after ordinary rollback and commit failure;
- staged identities as well as staged trees remaining observer-invisible;
- identity stability and uniqueness without assuming contiguous IDs.

The existing targeted in-memory and library tests pass, but they do not cover
these holes. No files were edited by the reviewer.
