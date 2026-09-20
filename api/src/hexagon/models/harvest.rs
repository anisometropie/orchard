use std::fmt;

use serde::{Deserialize, Serialize};

use super::{
    AnnualDate, AnnualHarvestWindow, HarvestScheduleOwner, HarvestedPart, OrchardId, OrchardTree,
    PlantIdentityId, TreeId,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct HarvestDate {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

impl HarvestDate {
    pub fn new(year: i32, month: u8, day: u8) -> Option<Self> {
        if !(1..=9999).contains(&year) || !(1..=12).contains(&month) {
            return None;
        }
        (day >= 1 && day <= days_in_month(year, month)).then_some(Self { year, month, day })
    }

    pub fn parse_iso(value: &str) -> Option<Self> {
        let bytes = value.as_bytes();
        if bytes.len() != 10
            || bytes[4] != b'-'
            || bytes[7] != b'-'
            || bytes
                .iter()
                .enumerate()
                .any(|(index, byte)| index != 4 && index != 7 && !byte.is_ascii_digit())
        {
            return None;
        }
        let year = value[0..4].parse().ok()?;
        let month = value[5..7].parse().ok()?;
        let day = value[8..10].parse().ok()?;
        Self::new(year, month, day)
    }

    pub fn add_days(self, days: u32) -> Option<Self> {
        let mut date = self;
        for _ in 0..days {
            let last_day = days_in_month(date.year, date.month);
            if date.day < last_day {
                date.day += 1;
            } else if date.month < 12 {
                date.month += 1;
                date.day = 1;
            } else {
                date = Self::new(date.year.checked_add(1)?, 1, 1)?;
            }
        }
        Some(date)
    }

    pub fn annual_date(self) -> AnnualDate {
        AnnualDate {
            month: self.month,
            day: self.day,
        }
    }
}

impl fmt::Display for HarvestDate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:04}-{:02}-{:02}",
            self.year, self.month, self.day
        )
    }
}

fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HarvestPeriod {
    pub start: HarvestDate,
    pub end: HarvestDate,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct HarvestRunId(pub u64);

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HarvestRunTarget {
    All,
    Species(PlantIdentityId),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HarvestTreeOutcome {
    HarvestedEverything {
        harvested_on: HarvestDate,
    },
    DoneForWindow {
        recorded_on: HarvestDate,
    },
    Deferred {
        deferred_on: HarvestDate,
        retry_on: HarvestDate,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HarvestRunTree {
    pub tree_id: TreeId,
    pub harvested_parts: Vec<HarvestedPart>,
    pub period: HarvestPeriod,
    pub outcome: Option<HarvestTreeOutcome>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HarvestRun {
    pub id: HarvestRunId,
    pub orchard_id: OrchardId,
    pub target: HarvestRunTarget,
    pub harvested_parts: Vec<HarvestedPart>,
    pub started_on: HarvestDate,
    pub ordered_trees: Vec<HarvestRunTree>,
    pub completed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompletedHarvestRun {
    pub id: HarvestRunId,
    pub target: HarvestRunTarget,
    pub harvested_parts: Vec<HarvestedPart>,
    pub started_on: HarvestDate,
    pub completed_at_unix_seconds: i64,
    pub ordered_trees: Vec<HarvestRunTree>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HarvestTreeOutcomeRecord {
    pub tree_id: TreeId,
    pub harvested_parts: Vec<HarvestedPart>,
    pub period: HarvestPeriod,
    pub outcome: HarvestTreeOutcome,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HarvestWindowExtension {
    pub owner: HarvestScheduleOwner,
    pub current_window: AnnualHarvestWindow,
    pub new_end: AnnualDate,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HarvestTreeActionUndo {
    pub tree_id: TreeId,
    pub previous_outcome: Option<HarvestTreeOutcome>,
    pub previous_period_end: HarvestDate,
    pub recorded_outcome: HarvestTreeOutcome,
    pub recorded_period_end: HarvestDate,
    pub window_extensions: Vec<HarvestWindowExtension>,
}

pub(crate) struct EligibleHarvestTree<'a> {
    pub tree: &'a OrchardTree,
    pub harvested_parts: Vec<HarvestedPart>,
    pub period: HarvestPeriod,
}

pub(crate) fn normalized_harvested_parts(
    mut harvested_parts: Vec<HarvestedPart>,
) -> Option<Vec<HarvestedPart>> {
    harvested_parts.sort_unstable();
    harvested_parts.dedup();
    (!harvested_parts.is_empty()).then_some(harvested_parts)
}

pub(crate) fn eligible_harvest_trees<'a>(
    orchard_trees: &'a [OrchardTree],
    outcomes: &[HarvestTreeOutcomeRecord],
    action_date: HarvestDate,
    selected_parts: &[HarvestedPart],
) -> Vec<EligibleHarvestTree<'a>> {
    orchard_trees
        .iter()
        .filter_map(|tree| {
            if !tree.tree.is_alive {
                return None;
            }
            let available_parts = selected_parts
                .iter()
                .filter_map(|harvested_part| {
                    current_contiguous_harvest_period(
                        &tree.harvest_windows,
                        *harvested_part,
                        action_date,
                    )
                    .map(|period| (*harvested_part, period))
                })
                .filter(|(harvested_part, period)| {
                    !outcomes.iter().any(|record| {
                        record.tree_id == tree.id
                            && record.harvested_parts.contains(harvested_part)
                            && periods_overlap(record.period, *period)
                            && match record.outcome {
                                HarvestTreeOutcome::HarvestedEverything { .. }
                                | HarvestTreeOutcome::DoneForWindow { .. } => true,
                                HarvestTreeOutcome::Deferred { retry_on, .. } => {
                                    action_date < retry_on
                                }
                            }
                    })
                })
                .collect::<Vec<_>>();
            let period = combined_period(&available_parts)?;
            Some(EligibleHarvestTree {
                tree,
                harvested_parts: available_parts
                    .into_iter()
                    .map(|(harvested_part, _)| harvested_part)
                    .collect(),
                period,
            })
        })
        .collect()
}

fn periods_overlap(left: HarvestPeriod, right: HarvestPeriod) -> bool {
    left.start <= right.end && right.start <= left.end
}

pub(crate) fn current_contiguous_harvest_period(
    windows: &[AnnualHarvestWindow],
    harvested_part: HarvestedPart,
    action_date: HarvestDate,
) -> Option<HarvestPeriod> {
    let mut periods = windows
        .iter()
        .filter(|window| window.harvested_part == harvested_part)
        .flat_map(|window| {
            (-1..=1).filter_map(move |offset| {
                action_date
                    .year
                    .checked_add(offset)
                    .and_then(|year| concrete_harvest_period(window, year))
            })
        })
        .collect::<Vec<_>>();
    periods.sort_by_key(|period| (period.start, period.end));

    let mut contiguous = Vec::<HarvestPeriod>::new();
    for period in periods {
        let Some(previous) = contiguous.last_mut() else {
            contiguous.push(period);
            continue;
        };
        let touches_previous = previous
            .end
            .add_days(1)
            .is_some_and(|day_after_previous| period.start <= day_after_previous);
        if touches_previous {
            if period.end > previous.end {
                previous.end = period.end;
            }
        } else {
            contiguous.push(period);
        }
    }
    contiguous
        .into_iter()
        .find(|period| period.start <= action_date && action_date <= period.end)
}

pub(crate) fn current_harvest_parts_and_period(
    windows: &[AnnualHarvestWindow],
    harvested_parts: &[HarvestedPart],
    action_date: HarvestDate,
) -> Option<(Vec<HarvestedPart>, HarvestPeriod)> {
    let available_parts = current_harvest_part_periods(windows, harvested_parts, action_date);
    let period = combined_period(&available_parts)?;
    Some((
        available_parts
            .into_iter()
            .map(|(harvested_part, _)| harvested_part)
            .collect(),
        period,
    ))
}

pub(crate) fn current_harvest_part_periods(
    windows: &[AnnualHarvestWindow],
    harvested_parts: &[HarvestedPart],
    action_date: HarvestDate,
) -> Vec<(HarvestedPart, HarvestPeriod)> {
    harvested_parts
        .iter()
        .filter_map(|harvested_part| {
            current_contiguous_harvest_period(windows, *harvested_part, action_date)
                .map(|period| (*harvested_part, period))
        })
        .collect()
}

fn combined_period(parts: &[(HarvestedPart, HarvestPeriod)]) -> Option<HarvestPeriod> {
    Some(HarvestPeriod {
        start: parts.iter().map(|(_, period)| period.start).min()?,
        end: parts.iter().map(|(_, period)| period.end).max()?,
    })
}

pub(crate) fn concrete_harvest_period(
    window: &AnnualHarvestWindow,
    start_year: i32,
) -> Option<HarvestPeriod> {
    let start = HarvestDate::new(start_year, window.start.month, window.start.day)?;
    let crosses_new_year =
        (window.end.month, window.end.day) < (window.start.month, window.start.day);
    let end_year = if crosses_new_year {
        start_year.checked_add(1)?
    } else {
        start_year
    };
    let end = HarvestDate::new(end_year, window.end.month, window.end.day)?;
    Some(HarvestPeriod { start, end })
}

#[cfg(test)]
mod tests {
    use super::HarvestDate;

    #[test]
    fn parse_only_real_strict_iso_dates() {
        assert_eq!(
            HarvestDate::parse_iso("2026-09-17"),
            HarvestDate::new(2026, 9, 17)
        );
        for invalid in [
            "2026-9-17",
            "2026-09-7",
            "2026/09/17",
            "0000-01-01",
            "2026-02-29",
            "2026-13-01",
            "not-a-date",
        ] {
            assert_eq!(HarvestDate::parse_iso(invalid), None, "{invalid}");
        }
        assert_eq!(
            HarvestDate::parse_iso("2024-02-29"),
            HarvestDate::new(2024, 2, 29)
        );
        assert_eq!(HarvestDate::new(2100, 2, 29), None);
        assert!(HarvestDate::new(2000, 2, 29).is_some());
    }

    #[test]
    fn add_days_across_month_year_and_leap_boundaries() {
        assert_eq!(
            HarvestDate::new(2026, 9, 27).unwrap().add_days(7),
            HarvestDate::new(2026, 10, 4)
        );
        assert_eq!(
            HarvestDate::new(2026, 12, 28).unwrap().add_days(7),
            HarvestDate::new(2027, 1, 4)
        );
        assert_eq!(
            HarvestDate::new(2024, 2, 25).unwrap().add_days(7),
            HarvestDate::new(2024, 3, 3)
        );
        assert_eq!(
            HarvestDate::new(2023, 2, 25).unwrap().add_days(7),
            HarvestDate::new(2023, 3, 4)
        );
    }
}
