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
| Not started | `TREE_CREATION_REQUESTED` | Create tree | A valid tree at a map coordinate is persisted. |
| Not started | `TREE_FIELD_SUGGESTIONS_REQUESTED` | Suggest tree fields | While entering a tree, an orchardist receives matching known species/cultivar suggestions from the fields already typed and may use suggested Latin name, roles, and harvest start/end days without retyping them. |
| Not started | `TREE_DETAILS_CHANGED` | Update tree | A selected tree's editable fields change without changing its identity. |
| Not started | `TREE_SEARCH_REQUESTED` | Search/list trees | Matching trees are returned for name, Latin name, role, harvest day, and danger filters. |
| Not started | `TREE_PHOTO_ATTACHED` | Attach tree photo | A valid image is stored and linked to the intended tree. |
| Not started | `WATERING_RUN_STARTED` | Start watering row | A watering session exists for one orchard row. |
| Not started | `NEXT_TREE_TO_WATER_REQUESTED` | Get next tree to water | The first unwatered tree in the row's defined order is returned. |
| Not started | `TREE_WATERED` | Record watering | One tree is recorded as watered in the active run and the next result advances. |
| Not started | `LEGACY_ORCHARD_IMPORT_REQUESTED` | Import legacy orchard | The existing GeoJSON trees and rows are migrated once with reviewed field mappings. |

## Known species/cultivars — future data requirement

Tree entries must not require the orchardist to repeatedly type botanical and
harvest information already known for a species or cultivar. The future
architecture needs a separate persisted species/cultivar catalog, distinct from
the trees planted in this orchard.

The front end will send the field values already entered by the orchardist to a
suggestion endpoint. It will return possible known species/cultivars and the
additional fields they can provide, initially including Latin name, tree roles,
and harvest start/end days. The front end presents these as suggestions; the
orchardist chooses whether to apply them. This is assistance, not an automatic
overwrite of entered values.

Do not design the table, endpoint shape, matching algorithm, or front-end
interaction until this use case is selected for its own reviewed TDD cycle.

## Technical adapter work, after its owning use case is green

- HTTP delivery adapter
- Postgres/PostGIS tree repository
- image-storage adapter
- Martin tile-source adapter/configuration
- database migration adapter

None of these adapters may contain an orchard decision.

## Deferred architecture decision

Do not introduce a transaction or unit-of-work abstraction yet. Revisit it
before implementing the first use case that must atomically change more than
one persistent record or resource, such as a tree plus a photo or audit event.
