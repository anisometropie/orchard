# PostgreSQL transaction review

Reviewer: `postgres_tx_review`

Verdict: the current PostgreSQL transaction implementation is not yet
production-sound. The happy path is correct, but important failure modes break
the transaction contract.

## High severity

### 1. Nested transactions break atomicity and adapter parity

`OrchardStorage::transaction()` passes `&mut Self`, so nesting is legal at
`api/src/hexagon/ports/orchard_storage.rs:14-19`. In-memory explicitly rejects
an active transaction at
`api/src/adapters/secondary/in_memory_orchard_storage/mod.rs:160-163`, but
PostgreSQL blindly sends raw `BEGIN` followed by `COMMIT` at
`api/src/adapters/secondary/postgres_orchard_storage/mod.rs:30-45`.

PostgreSQL treats `BEGIN` inside a transaction as a warning/no-op; the inner
`COMMIT` commits the whole outer transaction. If the outer closure then errors,
its `ROLLBACK` cannot undo those writes.

Add a PostgreSQL integration test equivalent to the in-memory nested
transaction rejection test, and reject nesting in PostgreSQL before issuing
SQL.

### 2. There is no panic-safe rollback

Raw `BEGIN`/`COMMIT` has no RAII or unwind guard at
`api/src/adapters/secondary/postgres_orchard_storage/mod.rs:30-46`. A panic in
`operation(self)` at line 33 bypasses both rollback branches, leaving the
persistent client in an open or possibly failed transaction.

In-memory also retains `Some(transaction)` on panic at
`api/src/adapters/secondary/in_memory_orchard_storage/mod.rs:163-168`.

Use private internal transaction state with unwind-safe cleanup, or a design
that can use `postgres::Transaction`, whose destructor rolls back. Test with
`catch_unwind`, then assert that no rows remain and storage is reusable.

### 3. Writes outside a transaction differ by adapter

PostgreSQL `find_or_create_plant_identity()` and `save_tree()` at
`api/src/adapters/secondary/postgres_orchard_storage/mod.rs:57-65` autocommit
when called without `transaction()`. In-memory rejects them because its
transaction is absent at
`api/src/adapters/secondary/in_memory_orchard_storage/mod.rs:232-235` and
`:267-270`.

The port exposes both methods without a state constraint. One omitted wrapper
in a future use case silently loses batch atomicity only in production.

Make PostgreSQL track and reject out-of-scope writes too. Defining both
adapters to auto-start each write would be consistent but would not support the
multi-call atomic use cases.

### 4. A swallowed SQL error can report success although PostgreSQL rolled everything back

After a statement error, PostgreSQL marks the transaction aborted. If the
closure catches or ignores, for example, a `save_tree()` error and returns
`Ok`, `COMMIT` on the aborted transaction completes as `ROLLBACK`.

The installed driver implementation confirms that
`tokio-postgres`'s `simple_query::batch_execute` ignores each
`CommandComplete` tag and returns `Ok` on `ReadyForQuery`. The current wrapper
therefore returns the callback value as a success even though nothing was
committed.

Current use cases propagate errors correctly, but the public port cannot
enforce that future callbacks will do so. Track or poison the active
transaction whenever a database storage method errors and refuse successful
commit, or make the transaction state own this invariant. Add an integration
test.

## Medium severity

### 5. Commit failure has an uncertain outcome

On connection loss around `COMMIT`, the server may have committed although the
client received an error. An attempted rollback cannot prove otherwise. The
CLI's generic import error at `api/src/bin/orchard.rs:112` says "No changes
were made," which is not guaranteed in this case.

Change the messaging and error semantics. Retrying is safe only with adequate
idempotency.

### 6. Rollback errors are discarded and a persistent client is reused

Rollback results are ignored at
`api/src/adapters/secondary/postgres_orchard_storage/mod.rs:36` and `:42`.
After cleanup, network, or protocol failure, storage can remain unusable or in
unknown transaction state with no invalidation or reconnection.

A connection pool with one checked-out connection per operation is the
conventional production shape because a bad connection can be discarded.

### 7. One PostgreSQL connection and a global mutex serialize the server

The HTTP server holds one storage object behind a global mutex at
`api/src/adapters/primary/http/mod.rs:33`, `:44`, `:82-87`, and `:105-121`.
Every read and write is serialized, and a long import, query, or failure blocks
the whole API.

This can be acceptable for a small personal deployment, but it is not
production-scale practice. Use a connection pool with a connection acquired
per operation; keep `spawn_blocking` if the synchronous driver remains, or use
an asynchronous PostgreSQL pool.

### 8. Transactional read parity is unspecified

PostgreSQL `trees()` called inside a transaction sees uncommitted rows on its
connection. In-memory `trees()` always reads committed state at
`api/src/adapters/secondary/in_memory_orchard_storage/mod.rs:283-308`.

Specify the contract and make in-memory merge staged state, or reject
`trees()` inside a transaction if read-your-writes is not intended.

## Low severity SQL semantics

- `legacy_feature_id as i32` and `PlantIdentityId as i64` silently wrap at
  `api/src/adapters/secondary/postgres_orchard_storage/mod.rs:177`,
  `:227-230`, and `:255`. Use checked conversions and map conversion failure.
- Concurrent legacy imports remain data-safe because of the database unique
  constraint, but check-then-insert races surface as `TreeCouldNotBeSaved`
  instead of `LegacyFeatureAlreadyImported`. Inspect the SQLSTATE or constraint
  name if that domain distinction matters.

## What is correct

- Happy-path transaction atomicity is real.
- Every operation within the callback uses the same PostgreSQL session.
- `is_legacy_tree_already_imported()` therefore sees staged rows.
- The plant identity unique key plus `ON CONFLICT` resolution is concurrency
  safe.
- The legacy ID unique constraint and foreign key protect database invariants.
- The integration rollback test verifies that both the staged plant identity
  and staged trees disappear after a propagated SQL error.

## Missing PostgreSQL tests

- nested transaction rejection;
- panic rollback and subsequent storage reuse;
- rejection of writes outside transactions;
- swallowed storage error behavior;
- reuse after ordinary rollback;
- concurrent legacy duplicate handling;
- commit and rollback failure behavior.

No files were edited by the reviewer.
