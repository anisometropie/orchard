# Database migrations

Add schema changes as `NNN_short_name.sql`. The build embeds every SQL file in
version order; adding a migration does not require editing Rust source.

Run migrations explicitly before starting application processes:

```sh
orchard migrate
```

Revert to an earlier version with:

```sh
orchard migrate revert --to VERSION
docker compose run --rm migrate revert --to VERSION
```

Version `0` removes the complete migrated schema. Down migrations live in
`db/migrations/down/` and must have exactly the same filename as their matching
up migration. Every active migration through version 014 has a down migration.
Versions 002 through 005 need no inverse SQL because their changes are already
present in the immutable version-001 baseline used by this runner.

Docker Compose does this through its one-shot `migrate` service. The API starts
only after that service succeeds.

Rules:

- migrations may transform existing application data only when required to
  preserve it under the new schema;
- applied migrations are immutable because their SHA-256 checksum is recorded;
- every migration and its ledger entry run in the same PostgreSQL transaction;
- a revert runs every selected down migration and ledger deletion, newest first,
  in one PostgreSQL transaction;
- a revert is refused before changing the database if any selected migration has
  no matching down migration;
- the migrator owns a database advisory lock for the complete run;
- do not use `BEGIN` or `COMMIT` in new files—the migrator supplies the transaction;
- a down migration must refuse a destructive conversion when the older schema
  cannot represent existing data;
- version `009` is permanently retired and rejected by the build.

The first run over the old untracked schema adopts it only when its structure
matches version 010 exactly. Empty databases are built from the migration chain.
Any partial or unexpected untracked schema is rejected without applying SQL.
