# Orchard access

Migrations `011_create_orchard_ownership` and
`012_add_orchard_authentication` are the complete prerequisite for orchard
login and read-only sharing. Migration 011 creates `My orchard` for the
existing default user and assigns that user's current trees, harvest windows,
and aerial overlays to it. Migration 012 adds password hashes, expiring
database sessions, and one rotatable share token per orchard.

Run the migrations, then set the existing user's password without placing it
in the command line:

```sh
docker compose run --rm migrate
ORCHARD_USER_PASSWORD='replace-with-a-long-password' \
  docker compose run --rm -e ORCHARD_USER_PASSWORD --entrypoint orchard migrate \
  set_user_password --username YOUR_USERNAME
docker compose up -d --build api nginx
```

Open `http://localhost:8080/`. An unlinked visit displays only the world map.
After login, the owner can open and modify only an orchard returned by their
session. “Share read-only link” rotates the orchard's previous link; anyone
with the new link can view that orchard, but write requests are rejected.

Session cookies are `Secure` by default. The Compose API service explicitly
sets `ORCHARD_ALLOW_INSECURE_HTTP=true` because the local/LAN endpoint uses
plain HTTP. Remove that override when serving behind HTTPS.
