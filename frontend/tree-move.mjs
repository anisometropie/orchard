export function idleTreeMoveState() {
  return {
    phase: "idle",
    treeId: null,
    treeName: null,
    originalPosition: null,
    proposedPosition: null,
    error: null,
  };
}

export function treeMoveTransition(state, event) {
  if (state.phase === "idle" && event.type === "USER_CLICKED_MOVE_TREE") {
    return {
      phase: "positioning",
      treeId: event.treeId,
      treeName: event.treeName,
      originalPosition: [...event.position],
      proposedPosition: [...event.position],
      error: null,
    };
  }
  if (
    state.phase === "positioning" &&
    event.type === "USER_DRAGGED_TREE_MOVE_PIN"
  ) {
    return {
      ...state,
      proposedPosition: [...event.position],
      error: null,
    };
  }
  if (
    state.phase === "positioning" &&
    event.type === "USER_CONFIRMED_TREE_MOVE"
  ) {
    return { ...state, phase: "saving", error: null };
  }
  if (
    state.phase === "positioning" &&
    event.type === "USER_CANCELLED_TREE_MOVE"
  ) {
    return idleTreeMoveState();
  }
  if (state.phase === "saving" && event.type === "TREE_MOVE_FAILED") {
    return { ...state, phase: "positioning", error: event.message };
  }
  if (state.phase === "saving" && event.type === "TREE_MOVE_SUCCEEDED") {
    return idleTreeMoveState();
  }
  return state;
}

export function treeMovePresentation(state) {
  return {
    active: state.phase !== "idle",
    interactionLocked: state.phase !== "idle",
    pin:
      state.phase === "idle"
        ? null
        : {
            position: [...state.proposedPosition],
            draggable: state.phase === "positioning",
          },
    confirmEnabled: state.phase === "positioning",
    cancelEnabled: state.phase === "positioning",
    saving: state.phase === "saving",
    error: state.error,
  };
}

export function treeMoveInteractionPolicy(state) {
  const active = state.phase !== "idle";
  return {
    backgroundControlsInert: active,
    mapGesturesEnabled: !active,
    scrimInterceptsPointer: false,
  };
}

export function treeMoveRequest(state) {
  if (state.phase !== "saving") return null;
  return {
    treeId: state.treeId,
    longitude: state.proposedPosition[0],
    latitude: state.proposedPosition[1],
  };
}
