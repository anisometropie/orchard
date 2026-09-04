# Rust API rebuild inventory

The former Python API was deliberately removed. This file is the retained
business contract for the Rust rebuild; it is not an implementation plan.

## Review rule

Implement exactly one use case at a time. Before any Rust production code,
write one realistic failing use-case test with in-memory adapters that implement
the same ports as the production adapters. Review its Red evidence before
writing the smallest Green implementation.

## Use cases to rebuild

| Status | Event | Use case | Observable outcome |
| --- | --- | --- | --- |
| Implemented | `TREE_CREATION_REQUESTED` | Create tree | A valid tree at a map coordinate is persisted with a reusable plant identity. |
| Implemented | `TREE_CONDITION_CHANGED` | Change tree condition | Any supplied danger/life fields change atomically without changing another tree; marking a tree dead clears danger. |
| Implemented | `MAP_CONFIGURATION_REQUESTED` | Load map configuration | The default user's center and aerial-overlay placement are loaded from persistent storage. |
| Implemented | `AERIAL_OVERLAY_IMAGE_REQUESTED` | Load aerial overlay image | An aerial overlay's image bytes and media type are loaded from persistent storage. |
| Implemented | `USER_LOGIN_REQUESTED` | Authenticate user | An Argon2 password creates a hashed, expiring database session before private orchard data or writes are available. |
| Implemented | `ORCHARD_SHARE_LINK_REQUESTED` | Share orchard read-only | An owner rotates a random share token that grants read-only access to exactly one orchard. |
| Not started | `TREE_FIELD_SUGGESTIONS_REQUESTED` | Suggest tree fields | While entering a tree, an orchardist receives matching known species/cultivar suggestions from the fields already typed and may use suggested Latin name, roles, and harvest start/end days without retyping them. |
| Not started | `TREE_DETAILS_CHANGED` | Update tree | A selected tree's editable fields change without changing its identity. |
| Not started | `TREE_SEARCH_REQUESTED` | Search/list trees | Matching trees are returned for name, Latin name, role, harvest day, and danger filters. |
| Not started | `TREE_PHOTO_ATTACHED` | Attach tree photo | A valid image is stored and linked to the intended tree. |
| Not started | `WATERING_RUN_STARTED` | Start watering row | A watering session exists for one orchard row. |
| Not started | `NEXT_TREE_TO_WATER_REQUESTED` | Get next tree to water | The first unwatered tree in the row's defined order is returned. |
| Not started | `TREE_WATERED` | Record watering | One tree is recorded as watered in the active run and the next result advances. |
| Implemented | `LEGACY_ORCHARD_IMPORT_REQUESTED` | Import legacy orchard | The existing GeoJSON trees and rows are migrated atomically with reviewed taxonomy mappings. |

## Privacy boundary

Map centers, aerial-overlay images/coordinates, trees, and mutations
are scoped by orchard. Owner access requires an expiring database-backed
session. A revocable share token grants read-only access to one orchard; the
frontend keeps that token in the URL fragment and sends it only in an API
header. A visit without an owner route or share fragment receives no orchard
data.

## Plant identity catalog

`PlantIdentity` is now a persisted catalog record, separate from planted
`Tree` records. A tree stores `plant_identity_id`; it does not copy normal
botanical, cultivar, or trade-name fields.

The catalog currently holds:

- canonical common name;
- structured botanical taxon, including varieties, subspecies, aggregates,
  cultivar groups, named hybrids, and hybrid formulae;
- one cultivar field;
- optional trade name; and
- identification confidence.

Legacy `name` and `latin_name` are retained only as raw import provenance on
the individual tree. An optional historical name/Latin pair is retained when
the source has one, as is an optional supplier/reference URL. They are not
normal frontend display fields.

Each imported tree also retains its tree-specific reproductive role when known:
`female`, `male`, `self_fertile`, or `parthenocarpic`. It is deliberately not a
catalog identity field yet; a future suggestion/editing use case can decide
whether and how a cultivar-level default should work.

The import and create-tree use cases find or create the catalog identity inside
their transaction. Reuse is based on botanical taxon, cultivar, and confidence;
legacy/common labels do not create duplicate catalog records.

## Suggestions — future use case

The front end will send the field values already entered by the orchardist to a
suggestion endpoint. It will return possible known species/cultivars and the
additional fields they can provide, initially including Latin name, tree roles,
and harvest start/end days. The front end presents these as suggestions; the
orchardist chooses whether to apply them. This is assistance, not an automatic
overwrite of entered values.

Do not design the table, endpoint shape, matching algorithm, or front-end
interaction until this use case is selected for its own reviewed TDD cycle.

## Technical adapter work

- HTTP create-tree delivery adapter
- Postgres/PostGIS orchard storage and migrations
- image-storage adapter
- Martin tile-source adapter/configuration

None of these adapters may contain an orchard decision.

## Transaction boundary

The existing orchard unit of work atomically stages/commits a plant identity and
tree together. It is intentionally limited to this one storage family.

When the first real multi-storage operation arrives—likely tree + photo + audit
event—stop and design that transaction boundary from that concrete use case.

## Existing populated database — separate migration use case

Migration `002` preserves old tree rows and makes their catalog reference
nullable. It cannot derive a reviewed plant identity from their old raw labels,
and the legacy importer correctly rejects already-imported feature IDs. An
already-populated database therefore needs a dedicated, test-first backfill or
fresh-reimport use case before it is used by a read/list endpoint.
