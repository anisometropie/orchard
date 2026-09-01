# Database migrations

Add schema changes as `NNN_short_name.sql`. The build embeds every SQL file in
version order; adding a migration does not require editing Rust source.

Run migrations explicitly before starting application processes:

```sh
orchard migrate
```

Docker Compose does this through its one-shot `migrate` service. The API starts
only after that service succeeds.

Rules:

- migrations contain schema transformations, not application or orchard data;
- applied migrations are immutable because their SHA-256 checksum is recorded;
- every migration and its ledger entry run in the same PostgreSQL transaction;
- the migrator owns a database advisory lock for the complete run;
- do not use `BEGIN` or `COMMIT` in new files—the migrator supplies the transaction;
- version `009` is permanently retired and rejected by the build.

The first run over the old untracked schema adopts it only when its structure
matches version 010 exactly. Empty databases are built from the migration chain.
Any partial or unexpected untracked schema is rejected without applying SQL.
