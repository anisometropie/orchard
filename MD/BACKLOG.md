# Backlog

## Replace the monolithic frontend with React and Redux

**Priority:** High  
**Status:** Backlog

### Problem

The frontend is concentrated in `frontend/index.html`, where markup, styling,
MapLibre integration, API calls, application state, and business behaviour are
coupled. Changes are difficult to isolate, test, and review.

### Target architecture

- React is the primary UI adapter.
- Redux is the single source of truth for frontend application state.
- Business rules and use cases live inside a framework-independent hexagon.
- API, browser, and MapLibre details remain in adapters outside the hexagon.
- The composition root creates the store and wires concrete adapters to ports.
- Redux actions describe events that happened; effect-oriented actions such as
  `SET_*`, `UPDATE_*`, `ADD_*`, and `REMOVE_*` are forbidden.
- Each use case owns a folder containing its implementation and outside-in tests.
- In-memory and production adapters implement the same ports; tests import test
  adapters from `adapters/secondary` instead of declaring fakes inline.

Suggested structure:

```text
frontend/src/
  hexagon/
    models/
    ports/
    reducers/
    use-cases/
  store/
  adapters/
    primary/react/
    secondary/http/
  bootstrap/
```

### Delivery approach

Migrate incrementally by vertical behaviour slice instead of replacing the
entire frontend at once:

1. Characterize the existing visible behaviour with focused browser tests.
2. Add the React, Redux, and test toolchain without changing behaviour.
3. Move existing pure inventory, harvest-window, and map-mode logic into the
   hexagon while preserving their tests.
4. Define ports and secondary HTTP adapters for orchard data and mutations.
5. Migrate one behaviour at a time using strict red-green-refactor cycles:
   map modes, map filters, inventory browsing, tree editing, then harvest-window
   editing.
6. Remove the legacy inline implementation only after the replacement slice has
   equivalent passing use-case and adapter-level coverage.

### Acceptance criteria

- `frontend/index.html` is only an application shell; it contains no application
  behaviour or large inline style/script blocks.
- React components contain presentation and event translation, not business
  rules.
- Every state change originates from an event-named Redux action.
- Use cases and reducers can be tested without React, the DOM, MapLibre, or a
  running API.
- Primary-adapter tests cover critical UI wiring that pure-data tests cannot
  prove, including map modes, expandable cultivar rows, and edit/cancel flows.
- HTTP adapter contract tests cover successful reads/writes and API failures.
- Existing user-visible behaviour remains available throughout the migration.
- Each production behaviour is introduced only after its observable test has
  failed for the expected reason, and all accumulated constraints remain green.
