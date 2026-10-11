use super::HarvestObservation;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HarvestWindowEvidence {
    StartTooEarly,
    StartTooLate,
    StartMayBeTooLate,
    EndTooEarly,
    HarvestOverWithinWindow,
    ReadyWithinWindow,
    EndConsistent,
    EndConfirmedByEarlierVisit,
    HarvestOverByVisit,
    LaterFruitPossible,
    MixedRipeness,
    EarlierFruitMissed,
    GapAfterEarlierHarvest,
    NotReadyYet,
}

/// Assess the saved visit against its saved window, never against subsequently edited dates.
/// Mixed fruit states are evidence about different parts of the crop, not a whole-tree timing label.
pub fn harvest_window_evidence(
    point: &HarvestObservation,
    history: &[HarvestObservation],
) -> Vec<HarvestWindowEvidence> {
    use HarvestWindowEvidence::*;
    if !point.has_timing_evidence() {
        return vec![];
    }
    let Some(period) = point.period else { return vec![]; };
    let fruit = point.observed_fruit_state();
    let mut evidence = vec![];
    let earlier_ready = history.iter().any(|earlier| {
        earlier.tree_id == point.tree_id
            && earlier.orchard_id == point.orchard_id
            && earlier.owner == point.owner
            && earlier.recorded_on < point.recorded_on
            && earlier.period == point.period
            && earlier
                .harvested_parts
                .iter()
                .any(|part| point.harvested_parts.contains(part))
            && earlier.observed_fruit_state().ready
    });
    if fruit.ready {
        evidence.push(if point.recorded_on < period.start {
            StartTooLate
        } else if point.recorded_on > period.end {
            EndTooEarly
        } else {
            ReadyWithinWindow
        });
    }
    if fruit.unripe && !fruit.ready && !fruit.wasted {
        evidence.push(if point.recorded_on < period.start {
            NotReadyYet
        } else if point.recorded_on >= period.end {
            LaterFruitPossible
        } else if earlier_ready {
            GapAfterEarlierHarvest
        } else {
            StartTooEarly
        });
    } else if fruit.unripe && point.recorded_on >= period.end {
        evidence.push(LaterFruitPossible);
    }
    if fruit.wasted {
        if fruit.ready || fruit.unripe {
            if point.recorded_on <= period.start {
                evidence.push(StartMayBeTooLate);
            }
            if !fruit.ready {
                evidence.push(EarlierFruitMissed);
            }
        } else if point.recorded_on <= period.start {
            evidence.push(StartTooLate);
        } else if point.recorded_on < period.end {
            evidence.push(HarvestOverWithinWindow);
        } else if point.recorded_on == period.end {
            evidence.push(if earlier_ready {
                EndConfirmedByEarlierVisit
            } else {
                EndConsistent
            });
        } else {
            evidence.push(HarvestOverByVisit);
        }
    }
    if u8::from(fruit.ready) + u8::from(fruit.unripe) + u8::from(fruit.wasted) > 1 {
        evidence.push(MixedRipeness);
    }
    evidence
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hexagon::models::*;

    fn point(date: &str, fruit_state: HarvestFruitState) -> HarvestObservation {
        HarvestObservation {
            id: 1,
            orchard_id: OrchardId(7),
            harvest_run_id: Some(HarvestRunId(1)),
            tree_id: TreeId(1),
            owner: HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1)),
            recorded_on: HarvestDate::parse_iso(date).unwrap(),
            kind: HarvestObservationKind::FruitRemaining,
            fruit_state: Some(fruit_state),
            harvested_parts: vec![HarvestedPart::Fruit],
            period: Some(HarvestPeriod {
                start: HarvestDate::new(2026, 9, 1).unwrap(),
                end: HarvestDate::new(2026, 9, 30).unwrap(),
            }),
            reviewed: false,
        }
    }
    #[test]
    fn assess_visits_at_start_inside_end_and_outside_the_saved_window() {
        use HarvestWindowEvidence::*;
        let ready = HarvestFruitState {
            ready: true,
            ..Default::default()
        };
        let unripe = HarvestFruitState {
            unripe: true,
            ..Default::default()
        };
        let wasted = HarvestFruitState {
            wasted: true,
            ..Default::default()
        };
        for (date, fruit, expected) in [
            ("2026-09-01", unripe, StartTooEarly),
            ("2026-09-01", wasted, StartTooLate),
            ("2026-09-15", ready, ReadyWithinWindow),
            ("2026-09-15", wasted, HarvestOverWithinWindow),
            ("2026-09-30", wasted, EndConsistent),
            ("2026-09-30", unripe, LaterFruitPossible),
            ("2026-08-31", ready, StartTooLate),
            ("2026-08-31", unripe, NotReadyYet),
            ("2026-10-01", ready, EndTooEarly),
            ("2026-10-01", wasted, HarvestOverByVisit),
        ] {
            assert_eq!(
                harvest_window_evidence(&point(date, fruit), &[]),
                vec![expected]
            );
        }
        let mut empty = point("2026-09-01", HarvestFruitState::default());
        empty.kind = HarvestObservationKind::EmptyOnArrival;
        assert!(harvest_window_evidence(&empty, &[]).is_empty());
    }
    #[test]
    fn mixed_stages_do_not_turn_the_entire_crop_into_a_too_late_observation() {
        use HarvestWindowEvidence::*;
        let mixed = point(
            "2026-09-30",
            HarvestFruitState {
                ready: true,
                unripe: true,
                wasted: true,
            },
        );
        assert_eq!(
            harvest_window_evidence(&mixed, &[]),
            vec![ReadyWithinWindow, LaterFruitPossible, MixedRipeness]
        );
        let point = point(
            "2026-09-15",
            HarvestFruitState {
                unripe: true,
                wasted: true,
                ready: false,
            },
        );
        assert_eq!(
            harvest_window_evidence(&point, &[]),
            vec![EarlierFruitMissed, MixedRipeness]
        );
    }
    #[test]
    fn earlier_ready_evidence_only_confirms_the_same_tree_parts_and_window() {
        use HarvestWindowEvidence::*;
        let earlier = point(
            "2026-09-15",
            HarvestFruitState {
                ready: true,
                ..Default::default()
            },
        );
        let last = point(
            "2026-09-30",
            HarvestFruitState {
                wasted: true,
                ..Default::default()
            },
        );
        assert_eq!(
            harvest_window_evidence(&last, &[earlier.clone()]),
            vec![EndConfirmedByEarlierVisit]
        );
        let gap = point(
            "2026-09-20",
            HarvestFruitState {
                unripe: true,
                ..Default::default()
            },
        );
        assert_eq!(
            harvest_window_evidence(&gap, &[earlier.clone()]),
            vec![GapAfterEarlierHarvest]
        );
        let mut other = earlier.clone();
        other.tree_id = TreeId(2);
        let mut other_window = earlier.clone();
        other_window.period.as_mut().unwrap().end = HarvestDate::new(2026, 10, 1).unwrap();
        let mut other_part = earlier;
        other_part.harvested_parts = vec![HarvestedPart::Flower];
        for history in [other, other_window, other_part] {
            assert_eq!(
                harvest_window_evidence(&last, &[history]),
                vec![EndConsistent]
            );
        }
    }
}
