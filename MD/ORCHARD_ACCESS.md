# Orchard access

Migrations `011_create_orchard_ownership` and
`012_add_orchard_authentication` are the complete prerequisite for orchard
login and read-only sharing. Migration 011 creates `My orchard` for the
existing default user and assigns that user's current trees, harvest windows,
and aerial overlays to it. Migration 012 adds password hashes and expiring
database sessions. Migrations 016 and 017 add separate view/watering
permissions and permanent share tokens; creating another link does not
invalidate existing links.

Run the migrations, then set the existing user's password without placing it
in the command line:

```sh
docker compose run --rm migrate
(
  read -rsp "New owner password: " ORCHARD_USER_PASSWORD
  printf '\n'
  export ORCHARD_USER_PASSWORD
  docker compose run --rm \
    -e ORCHARD_USER_PASSWORD \
    --entrypoint orchard \
    migrate set_user_password --username YOUR_USERNAME
)
```

Open `http://localhost:8080/`. An unlinked visit displays only the world map.
After login, the owner can open and modify only an orchard returned by their
session. Anyone with a view link can see that orchard, but write requests are
rejected. A watering link additionally permits the watering workflow.

See the root `README.md` for the complete VPS update, migration, nginx refresh,
and troubleshooting runbook.

Session cookies are `Secure` by default. The Compose API service explicitly
sets `ORCHARD_ALLOW_INSECURE_HTTP=true` because the local/LAN endpoint uses
plain HTTP. Remove that override when serving behind HTTPS.
