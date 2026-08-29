# Port and hexagonal architecture review

Reviewer: `port_arch_review`

Verdict: the architectural direction is sound, but the implementation is not
ready to call airtight yet. Do not restore `OrchardReader`,
`OrchardUnitOfWork`, or a public `OrchardTransaction`; the remaining problems
can be fixed while retaining one `OrchardStorage` port and private adapter
transaction state.

## High severity

### 1. PostgreSQL nesting can commit an outer transaction prematurely

`OrchardStorage::transaction()` can be called from inside its own callback at
`api/src/hexagon/ports/orchard_storage.rs:14`. In-memory rejects this at
`api/src/adapters/secondary/in_memory_orchard_storage/mod.rs:160`, but
PostgreSQL blindly sends another `BEGIN` at
`api/src/adapters/secondary/postgres_orchard_storage/mod.rs:30`.

PostgreSQL treats nested `BEGIN` as a warning/no-op; the inner `COMMIT` then
closes the actual outer transaction. A later outer error cannot roll it back.
[PostgreSQL documents that behavior](https://www.postgresql.org/docs/16/sql-begin.html).

PostgreSQL must track an active transaction and reject nesting before issuing
SQL, with the same contract test run against both adapters.

### 2. Writes outside a transaction have contradictory semantics

The port publicly exposes `find_or_create_plant_identity()` and `save_tree()`
independently at `api/src/hexagon/ports/orchard_storage.rs:25`.

- PostgreSQL autocommits both outside `transaction()` at
  `api/src/adapters/secondary/postgres_orchard_storage/mod.rs:57`.
- In-memory rejects a new identity at
  `api/src/adapters/secondary/in_memory_orchard_storage/mod.rs:232`, but an
  already-existing identity returns successfully before checking transaction
  state at line 221.
- In-memory `save_tree()` performs some validation and then rejects at line
  267.

That violates substitutability and makes accidental partial writes possible in
production. Given the current use cases, enforce "writes require an active
transaction" identically in both adapters.

### 3. Panic cleanup regressed from the former RAII transaction

In-memory installs transaction state, invokes arbitrary code, and only clears
it afterward at
`api/src/adapters/secondary/in_memory_orchard_storage/mod.rs:163`. If the
callback panics and the caller catches it, staged state remains visible to
duplicate checks and every future transaction is rejected.

PostgreSQL similarly skips `ROLLBACK` if the callback panics at
`api/src/adapters/secondary/postgres_orchard_storage/mod.rs:33`, leaving its
persistent connection inside the transaction.

Cleanup needs to be unwind-safe, followed by resuming the panic.

## Medium severity

### 4. Transactional `trees()` reads differ

In-memory reads only committed vectors at
`api/src/adapters/secondary/in_memory_orchard_storage/mod.rs:283`. PostgreSQL
uses the active connection and therefore sees its own uncommitted inserts.

Normal transaction semantics favor read-your-writes, so the in-memory query
should combine committed and staged identities/trees. The observer should
remain committed-only.

### 5. The generic transaction shape is idiomatic, but the error conversions are unsafe

`E: From<OrchardStorageError>` at
`api/src/hexagon/ports/orchard_storage.rs:18` is a legitimate Rust
transaction-callback pattern. The problem is the catch-all conversion:

- Tree creation maps every unexpected storage error to
  `TreeCouldNotBeSaved` at
  `api/src/hexagon/use_cases/create_tree/mod.rs:63`.
- Import maps every unexpected storage error to
  `ExistingLegacyFeaturesCouldNotBeChecked` at
  `api/src/hexagon/use_cases/import_legacy_orchard/mod.rs:83`.

New variants silently acquire false meanings. At minimum, make these matches
exhaustive and introduce an honest unexpected-storage failure. A
transaction-boundary error wrapper would be more type-precise, but is not
required to retain the single-port design.

### 6. Adapter construction reports the wrong concept

A database connection failure is mapped to `AtomicOperationCouldNotBegin` at
`api/src/adapters/secondary/postgres_orchard_storage/mod.rs:15`, although no
atomic operation was requested. `connect()` is adapter/composition-root
behavior and should return a connection-specific adapter error.

### 7. Error-catching behavior inside a callback needs a contract

If callback code catches a failed storage operation and returns `Ok`, in-memory
can commit earlier staged writes. A PostgreSQL statement error generally aborts
the transaction, so `COMMIT` may effectively roll it back. Either declare every
storage-operation error terminal and track that state consistently, or define
savepoint semantics.

## Low severity and explicit trade-offs

- `OrchardStorage` is not object-safe because `transaction()` is generic and
  mentions `Self`. That is not currently a defect: every composition point
  uses static generic dispatch. It only matters if runtime-selected
  `dyn OrchardStorage` becomes a requirement.
- One broad repository port makes the list use case depend syntactically on
  write capabilities. This weakens interface segregation, but with two
  adapters and the explicit simplicity requirement, it is a reasonable
  trade-off.
- `list_orchard_trees()` returning the broad `OrchardStorageError` exposes
  impossible write/transaction variants. A use-case-specific list error would
  be cleaner, but this is not blocking.
- Tree creation tests cover success, identity reuse, and save rollback, but not
  begin failure, commit failure, or identity-resolution failure. Those changed
  mappings at `api/src/hexagon/use_cases/create_tree/mod.rs:54` need direct
  tests.

## What is sound

- The use cases own the atomic boundary.
- `find_or_create_plant_identity()` and `save_tree()` remain orchard-storage
  operations rather than public transaction-object operations.
- The concrete transaction representation stays private to the secondary
  adapter.
- Import is one atomic batch; create-tree atomically persists identity plus
  tree.
- The legacy duplicate check correctly reads committed and staged trees.
- Staged changes remain invisible to the in-memory observer.
- Ordinary callback errors and simulated commit failures discard staged
  writes.
- Publishing in-memory identities and trees occurs under one mutex.
- Use cases depend inward on the port; PostgreSQL and in-memory
  implementations remain under secondary adapters; tests remain colocated
  with their use cases.
- Keeping list-trees as a use case is correct even though it is currently a
  thin delegation.

## Verification performed by the reviewer

Targeted create/import/list/in-memory tests passed: 24 tests. The all-target
run reached the two HTTP tests, which failed only because the review sandbox
could not bind sockets. No files were edited by the reviewer.
