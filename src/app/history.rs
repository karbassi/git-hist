use crate::app::commit::Commit;
use crate::app::diff::Diff;
use anyhow::{ensure, Result};

pub struct TurningPoint<'a> {
    commit: Commit<'a>,
    diff: Diff<'a>,
    is_latest: Option<bool>,
    is_earliest: Option<bool>,
    index_of_history: Option<usize>,
}

impl<'a> TurningPoint<'a> {
    pub fn new(commit: Commit<'a>, diff: Diff<'a>) -> Self {
        Self {
            commit,
            diff,
            is_latest: None,
            is_earliest: None,
            index_of_history: None,
        }
    }

    pub fn is_latest(&self) -> bool {
        self.is_latest.unwrap()
    }

    pub fn is_earliest(&self) -> bool {
        self.is_earliest.unwrap()
    }

    pub fn commit(&self) -> &Commit<'_> {
        &self.commit
    }

    pub fn diff(&self) -> &Diff<'_> {
        &self.diff
    }
}

pub struct History<'a> {
    points: Vec<TurningPoint<'a>>,
}

impl<'a> History<'a> {
    pub fn new<I: Iterator<Item = TurningPoint<'a>>>(points: I) -> Result<Self> {
        let mut points = points
            .enumerate()
            .map(|(i, mut p)| {
                p.index_of_history = Some(i);
                p
            })
            .collect::<Vec<_>>();
        ensure!(
            !points.is_empty(),
            "No changes found for this file in the commit history"
        );

        let len = points.len();
        for point in points.iter_mut() {
            point.is_latest = Some(point.index_of_history.unwrap() == 0);
            point.is_earliest = Some(point.index_of_history.unwrap() + 1 == len);
        }
        Ok(History { points })
    }

    pub fn latest(&self) -> Option<&TurningPoint<'_>> {
        self.points.first()
    }

    pub fn backward(&self, point: &TurningPoint) -> Option<&TurningPoint<'_>> {
        point
            .index_of_history
            .and_then(|i| i.checked_add(1))
            .and_then(|i| self.points.get(i))
    }

    pub fn forward(&self, point: &TurningPoint) -> Option<&TurningPoint<'_>> {
        point
            .index_of_history
            .and_then(|i| i.checked_sub(1))
            .and_then(|i| self.points.get(i))
    }
}
