# Orchard storage transaction design audit

Audit date: 2026-08-29

This directory preserves the consolidated audit and the complete reports from
three independent reviews:

- [Port and hexagonal architecture review](./port-and-architecture-review.md)
- [In-memory transaction review](./in-memory-transaction-review.md)
- [PostgreSQL transaction review](./postgres-transaction-review.md)

No production files were changed as part of the audit.

## Consolidated verdict

The architectural direction is sound, but the implementation is not yet
airtight or production-grade.

The design should retain:

- one `OrchardStorage` port;
- transaction boundaries owned by application use cases;
- `find_or_create_plant_identity()` and `save_tree()` as orchard-storage
  operations;
- private transaction representations inside the secondary adapters;
- list-trees as an application use case;
- one atomic transaction for the complete legacy import and one atomic
  transaction for tree creation.

There is no demonstrated need to restore `OrchardReader`, expose a public
`OrchardTransaction`, or add another unit-of-work abstraction.

## Production blockers

### 1. Nested PostgreSQL transactions can commit prematurely

`OrchardStorage::transaction()` can be called recursively. The in-memory
adapter rejects that, but the PostgreSQL adapter sends another `BEGIN`.
PostgreSQL ignores the nested `BEGIN` while the inner `COMMIT` closes the real
outer transaction. A later outer rollback therefore cannot undo the writes.

References:

- `api/src/hexagon/ports/orchard_storage.rs:14`
- `api/src/adapters/secondary/in_memory_orchard_storage/mod.rs:160`
- `api/src/adapters/secondary/postgres_orchard_storage/mod.rs:30`

### 2. Writes outside `transaction()` violate adapter substitutability

PostgreSQL autocommits `find_or_create_plant_identity()` and `save_tree()` when
they are called outside `transaction()`. In-memory rejects new identities and
tree writes, but can return an existing identity successfully before checking
whether a transaction is active.

One forgotten transaction wrapper could consequently create partial writes in
production while the in-memory tests behave differently. Both adapters should
reject writes unless a transaction is active.

References:

- `api/src/hexagon/ports/orchard_storage.rs:25`
- `api/src/adapters/secondary/in_memory_orchard_storage/mod.rs:221`
- `api/src/adapters/secondary/in_memory_orchard_storage/mod.rs:232`
- `api/src/adapters/secondary/in_memory_orchard_storage/mod.rs:267`
- `api/src/adapters/secondary/postgres_orchard_storage/mod.rs:57`

### 3. Panics leave unusable transaction state

A panic inside the callback skips cleanup. In-memory retains its staged
transaction and rejects every future transaction. PostgreSQL leaves its
persistent connection inside an open or aborted transaction. Cleanup must be
unwind-safe and the original panic must then resume.

References:

- `api/src/adapters/secondary/in_memory_orchard_storage/mod.rs:163`
- `api/src/adapters/secondary/postgres_orchard_storage/mod.rs:33`

### 4. PostgreSQL can report success after rolling everything back

After a PostgreSQL statement error, the transaction is aborted. If callback
code catches that error and returns `Ok`, `COMMIT` completes as `ROLLBACK`.
The driver's `batch_execute()` ignores the command-completion tag and may
return `Ok`, causing `transaction()` to report success although nothing was
committed.

The transaction must be marked failed whenever a storage operation fails, or
the API must otherwise make it impossible to convert an aborted transaction
into a successful result.

## Important parity and operational issues

- `trees()` reads staged writes in PostgreSQL but only committed writes in the
  in-memory adapter.
- If callback code catches a storage error, in-memory may commit earlier staged
  work while PostgreSQL normally aborts the entire transaction.
- In-memory identity IDs are inferred from vector positions and therefore
  assume contiguous IDs; PostgreSQL sequences legitimately contain gaps.
- PostgreSQL rollback failures are discarded and the same potentially broken
  persistent connection is reused.
- A failure around `COMMIT` has an uncertain outcome. User-facing code must not
  promise that no changes were made in that case.
- One synchronous PostgreSQL client behind the HTTP storage mutex serializes
  every request. This can be acceptable for a small personal deployment, but
  it is not a scalable production topology.
- Catch-all conversions in use-case error mappings can silently give new
  storage errors the wrong business meaning.
- `OrchardStorage` is not object-safe because `transaction()` is generic. This
  is an explicit, currently harmless trade-off because composition uses static
  generic dispatch.

## Confirmed strengths

- The use cases own the atomic boundary.
- The concrete transaction representation remains private to each adapter.
- The legacy import is one atomic batch.
- Tree creation atomically persists the identity and tree.
- Legacy duplicate detection reads committed and staged trees.
- Staged in-memory changes remain invisible to external observers.
- Ordinary callback errors and simulated commit failures discard staged
  in-memory changes.
- In-memory identities and trees are published under one mutex.
- PostgreSQL operations within a transaction use the same session and see
  their own uncommitted changes.
- PostgreSQL uniqueness constraints and foreign keys reinforce invariants.
- Identity resolution through `ON CONFLICT` is concurrency-safe.
- Use cases depend inward on the port and both storage implementations remain
  secondary adapters.

## Required shared adapter contract tests

The same behavioral tests should run against both storage implementations:

1. nested transactions are rejected without persisting inner or outer writes;
2. writes outside an active transaction are rejected;
3. panic cleanup rolls back and leaves storage reusable;
4. a caught storage failure cannot produce a successful partial commit;
5. reads inside a transaction have explicitly defined read-your-writes
   behavior;
6. staged identities and trees remain invisible to external observers;
7. storage remains usable after ordinary rollback and commit failure;
8. identity behavior does not rely on contiguous numeric IDs;
9. concurrent legacy duplicates preserve database invariants;
10. rollback and commit failure behavior is surfaced honestly.

## Recommended implementation order

1. Add the shared adapter contract tests.
2. Enforce active-transaction-only writes in both adapters.
3. Reject nested PostgreSQL transactions before sending SQL.
4. Add unwind-safe cleanup and transaction failure/poison tracking.
5. Define and align transactional read semantics.
6. Correct identity modeling and error conversions.
7. Introduce connection acquisition/pooling and broken-connection recovery if
   the deployment needs concurrent production traffic.
