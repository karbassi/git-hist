use crate::app::commit::Commit;
use crate::app::diff::Diff;
use anyhow::{ensure, Result};

pub struct TurningPoint<'a> {
    commit: Commit<'a>,
    diff: Diff<'a>,
    is_latest: bool,
    is_earliest: bool,
    index_of_history: usize,
}

impl<'a> TurningPoint<'a> {
    pub fn new(commit: Commit<'a>, diff: Diff<'a>) -> Self {
        Self {
            commit,
            diff,
            is_latest: false,
            is_earliest: false,
            index_of_history: 0,
        }
    }

    pub fn is_latest(&self) -> bool {
        self.is_latest
    }

    pub fn is_earliest(&self) -> bool {
        self.is_earliest
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
                p.index_of_history = i;
                p
            })
            .collect::<Vec<_>>();
        ensure!(
            !points.is_empty(),
            "No changes found for this file in the commit history"
        );

        let len = points.len();
        for point in points.iter_mut() {
            point.is_latest = point.index_of_history == 0;
            point.is_earliest = point.index_of_history + 1 == len;
        }
        Ok(History { points })
    }

    pub fn latest(&self) -> Option<&TurningPoint<'_>> {
        self.points.first()
    }

    pub fn backward(&self, point: &TurningPoint) -> Option<&TurningPoint<'_>> {
        point
            .index_of_history
            .checked_add(1)
            .and_then(|i| self.points.get(i))
    }

    pub fn forward(&self, point: &TurningPoint) -> Option<&TurningPoint<'_>> {
        point
            .index_of_history
            .checked_sub(1)
            .and_then(|i| self.points.get(i))
    }
}
