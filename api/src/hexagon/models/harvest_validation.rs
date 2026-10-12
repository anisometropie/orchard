use super::{
    AnnualDate, AnnualHarvestWindow, HarvestObservation, HarvestWindowEvidence, HarvestedPart,
    harvest_window_evidence,
};
use serde::Serialize;

#[derive(Clone, Copy)]
pub struct HarvestWindowSelection {
    pub start: AnnualDate,
    pub end: AnnualDate,
    pub harvested_part: HarvestedPart,
}

impl HarvestWindowSelection {
    pub fn matches(self, window: &AnnualHarvestWindow) -> bool {
        self.start == window.start
            && self.end == window.end
            && self.harvested_part == window.harvested_part
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub struct HarvestValidationEvidence {
    pub observation_count: usize,
    pub start_supported: bool,
    pub end_supported: bool,
    pub conflicting_observations: usize,
}

/// Advisory evidence only. Validation always remains an explicit owner decision.
/// Bracket each boundary within a weekly visit interval, using the same tree and season.
pub fn harvest_validation_evidence(
    window: &AnnualHarvestWindow,
    points: &[HarvestObservation],
) -> HarvestValidationEvidence {
    let scoped = points
        .iter()
        .filter(|point| {
            point.has_timing_evidence()
                && point.harvested_parts.contains(&window.harvested_part)
                && point.period.is_some_and(|period| {
                    period.start.annual_date() == window.start
                        && period.end.annual_date() == window.end
                })
        })
        .collect::<Vec<_>>();
    let pair = |first: &&HarvestObservation, second: &&HarvestObservation| {
        first.tree_id == second.tree_id
            && first.orchard_id == second.orchard_id
            && first.owner == second.owner
            && first.period == second.period
            && first.recorded_on < second.recorded_on
    };
    let start_supported = scoped.iter().any(|first| {
        let fruit = first.observed_fruit_state();
        let start = first.period.unwrap().start;
        fruit.unripe
            && !fruit.ready
            && !fruit.wasted
            && first.recorded_on <= start
            && first
                .recorded_on
                .add_days(7)
                .is_some_and(|date| date >= start)
            && scoped.iter().any(|second| {
                pair(first, second)
                    && second.observed_fruit_state().ready
                    && second.recorded_on >= start
                    && start
                        .add_days(7)
                        .is_some_and(|date| second.recorded_on <= date)
            })
    });
    let end_supported = scoped.iter().any(|first| {
        let end = first.period.unwrap().end;
        first.observed_fruit_state().ready
            && first.recorded_on <= end
            && first
                .recorded_on
                .add_days(7)
                .is_some_and(|date| date >= end)
            && scoped.iter().any(|second| {
                let fruit = second.observed_fruit_state();
                pair(first, second)
                    && fruit.wasted
                    && !fruit.ready
                    && !fruit.unripe
                    && second.recorded_on >= end
                    && end
                        .add_days(7)
                        .is_some_and(|date| second.recorded_on <= date)
            })
    });
    let conflicting_observations = scoped
        .iter()
        .filter(|point| {
            harvest_window_evidence(point, points)
                .iter()
                .any(|evidence| {
                    matches!(
                        evidence,
                        HarvestWindowEvidence::StartTooEarly
                            | HarvestWindowEvidence::StartTooLate
                            | HarvestWindowEvidence::StartMayBeTooLate
                            | HarvestWindowEvidence::EndTooEarly
                            | HarvestWindowEvidence::HarvestOverWithinWindow
                            | HarvestWindowEvidence::LaterFruitPossible
                    )
                })
        })
        .count();
    HarvestValidationEvidence {
        observation_count: scoped.len(),
        start_supported,
        end_supported,
        conflicting_observations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hexagon::models::*;

    fn window() -> AnnualHarvestWindow {
        AnnualHarvestWindow {
            start: AnnualDate { month: 9, day: 1 },
            end: AnnualDate { month: 9, day: 30 },
            harvested_part: HarvestedPart::Fruit,
            reference_region: None,
            data_origin: HarvestDataOrigin::FieldObservation,
            source_url: None,
            validated: false,
        }
    }
    fn point(date: &str, kind: HarvestObservationKind) -> HarvestObservation {
        HarvestObservation {
            id: 1,
            orchard_id: OrchardId(7),
            tree_id: TreeId(1),
            owner: HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1)),
            harvest_run_id: None,
            recorded_on: HarvestDate::parse_iso(date).unwrap(),
            kind,
            fruit_state: if kind == HarvestObservationKind::FruitRemaining {
                Some(HarvestFruitState {
                    unripe: true,
                    ..Default::default()
                })
            } else {
                None
            },
            harvested_parts: vec![HarvestedPart::Fruit],
            period: Some(HarvestPeriod {
                start: HarvestDate::new(2026, 9, 1).unwrap(),
                end: HarvestDate::new(2026, 9, 30).unwrap(),
            }),
            reviewed: false,
        }
    }
    #[test]
    fn one_ready_visit_does_not_establish_either_boundary() {
        assert_eq!(
            harvest_validation_evidence(
                &window(),
                &[point("2026-09-15", HarvestObservationKind::HarvestedAll)]
            ),
            HarvestValidationEvidence {
                observation_count: 1,
                start_supported: false,
                end_supported: false,
                conflicting_observations: 0
            }
        );
    }
    #[test]
    fn separate_transitions_near_start_and_end_support_both_boundaries() {
        let points = vec![
            point("2026-08-30", HarvestObservationKind::FruitRemaining),
            point("2026-09-02", HarvestObservationKind::HarvestedAll),
            point("2026-09-28", HarvestObservationKind::HarvestedAll),
            point("2026-10-01", HarvestObservationKind::AllWasted),
        ];
        assert_eq!(
            harvest_validation_evidence(&window(), &points),
            HarvestValidationEvidence {
                observation_count: 4,
                start_supported: true,
                end_supported: true,
                conflicting_observations: 0
            }
        );
        let mut unrelated = points.clone();
        unrelated[1].tree_id = TreeId(2);
        unrelated[3].period.as_mut().unwrap().start.year = 2025;
        let assessment = harvest_validation_evidence(&window(), &unrelated);
        assert!(!assessment.start_supported && !assessment.end_supported);
        let mut mixed = points.clone();
        mixed[3].kind = HarvestObservationKind::FruitRemaining;
        mixed[3].fruit_state = Some(HarvestFruitState {
            ready: true,
            wasted: true,
            unripe: false,
        });
        let assessment = harvest_validation_evidence(&window(), &mixed);
        assert!(assessment.start_supported && !assessment.end_supported);
        assert_eq!(assessment.conflicting_observations, 1);
    }
    #[test]
    fn remote_dates_empty_visits_and_other_parts_do_not_validate_boundaries() {
        let points = vec![
            point("2026-08-01", HarvestObservationKind::FruitRemaining),
            point("2026-09-15", HarvestObservationKind::HarvestedAll),
            point("2026-10-20", HarvestObservationKind::AllWasted),
            point("2026-09-30", HarvestObservationKind::EmptyOnArrival),
        ];
        let evidence = harvest_validation_evidence(&window(), &points);
        assert_eq!(evidence.observation_count, 3);
        assert!(!evidence.start_supported && !evidence.end_supported);
        let mut other = points;
        for p in &mut other {
            p.harvested_parts = vec![HarvestedPart::Flower];
        }
        assert_eq!(
            harvest_validation_evidence(&window(), &other).observation_count,
            0
        );
    }
}
