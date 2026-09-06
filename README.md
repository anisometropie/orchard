# Orchard Map

This repository runs as PostgreSQL/PostGIS, a Rust API, and nginx serving the
frontend. Run the commands below from the repository root on the VPS.

## Safe VPS update on the 1 GB box

Take a database backup before applying a new migration:

```bash
mkdir -p backups
docker compose exec -T db pg_dump -U orchard -d orchard -Fc \
  > "backups/orchard-$(date +%F-%H%M%S).dump"
```

For an update containing a Rust API change and a new migration, use this order:

```bash
git pull --ff-only

# Build one service at a time to keep peak memory low.
docker compose build migrate

# Migration files are embedded in this newly built Rust binary.
docker compose run --rm migrate

# This should reuse everything compiled by the migrate build.
docker compose build api
docker compose up -d --no-deps api

# Recreate nginx so its generated config and frontend are current.
docker compose up -d --force-recreate nginx

docker compose ps -a
```

Do not start the new API before its migrations have succeeded.

## Which commands are needed?

| Changed files | Required action |
| --- | --- |
| Only `frontend/` | Pull and hard-refresh Chrome. Recreate nginx only if the old frontend remains visible. |
| `nginx.conf` | `docker compose up -d --force-recreate nginx` |
| Rust code under `api/src/`, without a migration | Build `api`, then recreate `api`. |
| A new file under `api/db/migrations/` | Build `migrate`, run it, then build and recreate `api`. |
| `api/Cargo.toml`, `api/Cargo.lock`, or `api/Dockerfile` | Expect a longer build; build one service at a time. |
| `docker-compose.yml` | Run `docker compose up -d` for affected services so Compose applies the new configuration. |

### API-only update without a migration

```bash
git pull --ff-only
docker compose build api
docker compose up -d --no-deps api
```

### Frontend-only update

The frontend directory is bind-mounted into nginx, so a pull normally updates
it without a Docker image build. First use Chrome's hard refresh
(`Ctrl+Shift+R`). If the old page is still served:

```bash
docker compose up -d --force-recreate nginx
```

If Chrome still shows the old version, try an incognito window or temporarily
disable the cache in Chrome DevTools. Check what the VPS is actually serving:

```bash
curl -I http://127.0.0.1:8080/
```

Do not run `docker compose build nginx`: this project uses the stock nginx image
and bind-mounts both `nginx.conf` and `frontend/`.

## Migrations

Migration SQL is embedded into the Rust executable at build time. A stale
`migrate` image cannot see a migration added by the latest pull.

Whenever a new migration exists:

```bash
docker compose build migrate
docker compose run --rm migrate
docker compose build api
docker compose up -d --no-deps api
```

Running the migrator again is safe. It validates applied migration names and
checksums and applies only missing versions:

```bash
docker compose run --rm migrate
```

To inspect the migration ledger directly:

```bash
docker compose exec -T db psql -U orchard -d orchard -c \
  'SELECT version, name, applied_at FROM orchard_schema_migrations ORDER BY version;'
```

### `applied migration N is unknown to this binary`

The database is newer than the `migrate` image being executed. Rebuild that
image from the current checkout, then retry:

```bash
git pull --ff-only
docker compose build migrate
docker compose run --rm migrate
```

Do not edit the migration ledger, rename an applied migration, or renumber old
migrations to work around this error. Applied migration names and checksums are
deliberately immutable.

## Initialize or change the owner password safely

The CLI command is `set_user_password`, but the Compose `migrate` service has
`orchard migrate` as its normal entrypoint. Override the entrypoint as shown
below. The password is read without echo, is not put in the command line or bash
history, and exists only inside a temporary subshell:

```bash
(
  read -rsp "New owner password: " ORCHARD_USER_PASSWORD
  printf '\n'
  export ORCHARD_USER_PASSWORD
  docker compose run --rm \
    -e ORCHARD_USER_PASSWORD \
    --entrypoint orchard \
    migrate set_user_password --username owner
)
```

Passwords must contain at least 12 characters. Changing a password revokes the
owner's existing login sessions.

Do not write the password directly after `-e`. `-e ORCHARD_USER_PASSWORD` means
“pass the already-set variable into the container.”

## Rust rebuilds and the 1 GB VPS

The API Dockerfile uses persistent BuildKit Cargo caches and compiles with
`--jobs 1`. Preserve those caches:

- Do not use `docker compose build --no-cache` for normal updates.
- Do not run `docker builder prune` or `docker system prune` casually. Removing
  the build cache forces Cargo to compile every dependency again.
- Build `migrate` and `api` sequentially on the 1 GB VPS. They share the same
  source and the second build should reuse the first build's cache.

A full dependency compilation is expected after the first build, a Rust image
or toolchain change, a dependency lockfile change, or a cache prune. A one-line
Rust source change should otherwise reuse compiled dependencies even though
Cargo still prints a short compilation/checking phase.

## Shared links

View and watering share links are permanent database records. Creating a new
link does not invalidate older links. Copy the complete URL, including the part
after `#`.

- View links can read the orchard and its photos.
- Watering links can also run watering workflows.
- Neither kind of shared link can edit trees, row order, harvest data, or upload
  photos.
- Only a logged-in orchard owner can create links or upload photos.

If a valid link is rejected after an update, verify that the latest migrations
and API image are running, then inspect the API logs:

```bash
docker compose run --rm migrate
docker compose logs --tail=100 api
```

## Common failures

### The new frontend is not visible

```bash
docker compose up -d --force-recreate nginx
```

Then hard-refresh Chrome. Rebuilding the Rust API does not refresh nginx.

### Photo upload returns HTTP 413

The running nginx container still has the old 1 MB request limit, or the photo
is genuinely too large. Pull the current `nginx.conf` and recreate nginx:

```bash
docker compose up -d --force-recreate nginx
```

The application stores an at-most 8 MB high-quality WebP and a 512 KB WebP
thumbnail. The encoded upload request is limited to 13 MB.

### A newly started watering run says it no longer exists

The browser, API image, and database schema may not all be from the same
deployment. Run the migration/update sequence, recreate the API, then reload:

```bash
docker compose build migrate
docker compose run --rm migrate
docker compose build api
docker compose up -d --no-deps api
docker compose logs --tail=100 api
```

### Compose warns about an orphan container

An orphan warning is informational. Do not add `--remove-orphans` unless the
named container is definitely obsolete: that flag stops and removes it. This is
especially relevant to the existing `research-worker` container.

### Basic service diagnosis

```bash
docker compose ps -a
docker compose logs --tail=100 api migrate nginx db
docker compose exec -T db pg_isready -U orchard -d orchard
curl -sS -o /dev/null -w 'frontend: %{http_code}\n' http://127.0.0.1:8080/
curl -sS -o /dev/null -w 'api session: %{http_code}\n' http://127.0.0.1:8080/api/session
```

The frontend should return `200`. An anonymous session request normally returns
`401`; that still proves nginx can reach the API.

## HTTPS

The current Compose setup exposes plain HTTP on port 8080. For public HTTPS you
need:

1. A domain whose DNS points to the VPS.
2. TCP ports 80 and 443 open in the VPS firewall/provider firewall.
3. A TLS reverse proxy such as Caddy or host nginx forwarding to
   `127.0.0.1:8080`.
4. `ORCHARD_ALLOW_INSECURE_HTTP` removed from the API environment, followed by
   recreating the API, so authentication cookies remain Secure.

Keep `ORCHARD_ALLOW_INSECURE_HTTP=true` only for intentional plain-HTTP LAN or
local use.

## Database backups

The PostgreSQL backup includes tree photos because they are stored in `BYTEA`.
To inspect their current database footprint:

```bash
docker compose exec -T db psql -U orchard -d orchard -c \
  "SELECT pg_size_pretty(pg_total_relation_size('tree_photos'));"
```

Restore procedures are intentionally not presented as a copy-paste command:
restoring overwrites live state. Confirm the target database and backup file
before performing a restore.
