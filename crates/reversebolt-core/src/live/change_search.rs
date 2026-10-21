use super::*;
use crate::change_search::{self as search, Summary};

impl LiveSession {
    pub fn collecting_change_search(&self) -> bool {
        self.view.change_searches.last().is_some_and(|s| {
            s.selection_batch == self.view.coverage.batch && s.status == "collecting"
        })
    }

    fn changes_restored(&self, store: &crate::changes::Store) -> Result<()> {
        if store.records.iter().any(|c| {
            c.scope.investigation == self.view.investigation_id
                && (c.phase.unresolved() || c.phase == crate::changes::Phase::Written)
        }) {
            return Err("Restore the applied trial before searching. Uncertain writes stay in their investigation.".into());
        }
        Ok(())
    }

    pub fn start_change_search(
        &mut self,
        investigation: &str,
        batch: usize,
        after_frame: usize,
        track: usize,
        store: &crate::changes::Store,
    ) -> Result<()> {
        self.check_batch_transition(investigation, batch)?;
        self.changes_restored(store)?;
        if self.collecting_change_search() {
            return Err("A change search is already collecting observations.".into());
        }
        if after_frame != self.view.sample_count || after_frame == 0 {
            return Err("The observed moment changed; refresh this card.".into());
        }
        if self.view.change_searches.len() >= search::MAX_SEARCHES
            || batch + 2 > coverage::MAX_BATCHES
        {
            return Err(
                "Not enough searches or memory batches remain for selection and fresh validation."
                    .into(),
            );
        }
        let observed = self
            .view
            .tracks
            .iter()
            .find(|t| t.id == track && t.active && t.samples.last().copied().flatten().is_some())
            .ok_or("Choose a currently readable number.")?;
        let frame = self
            .view
            .frames
            .last()
            .filter(|f| f.number == after_frame)
            .ok_or("The current image is unavailable.")?
            .clone();
        // Keep the existing transition's conservative twelve-read reservation,
        // plus room for twenty small fresh observations after selection.
        let reserve = (crate::value_search::MAX_WINDOWS
            * crate::value_search::WINDOW_BYTES
            * MAX_SAMPLES) as u64;
        let remaining = coverage::MAX_READ_BYTES
            .saturating_sub(self.view.coverage.attempted_bytes)
            .saturating_sub(reserve);
        let budget = (remaining / 12).min(search::MAX_BYTES as u64) as usize;
        if budget < coverage::CHUNK_BYTES {
            return Err(
                "Not enough read budget remains for this search and fresh observations.".into(),
            );
        }
        let mut excluded = self.selected.clone();
        for c in &self.plan {
            excluded.add(c.address, c.length);
        }
        let (plan, eligible) = coverage::plan(self.target.regions()?, &excluded, budget)?;
        let planned_bytes = plan.iter().map(|c| c.length).sum();
        if planned_bytes == 0 {
            return Err("No unvisited memory remains for this search.".into());
        }
        let number = self.view.change_searches.len() + 1;
        let summary = Summary {
            source_batch: batch,
            source_track: track,
            source_frame: frame,
            label: observed.label.clone(),
            bounds: observed.bounds,
            selection_batch: batch + 1,
            planned_bytes,
            status: "collecting".into(),
            reason: None,
            observations: vec![],
            basis: vec![],
            complete_bytes: 0,
            matched_windows: 0,
            retained_windows: 0,
            omitted_windows: 0,
            boundary_matches: 0,
            elapsed_ms: 0,
            evidence: self
                .root_directory
                .join(format!("change-search-{number:02}.json"))
                .to_string_lossy()
                .into_owned(),
        };
        let bytes=serde_json::to_vec(&serde_json::json!({"planned_search":summary,"scope":"Selection plan only; not evidence of a completed capture or transition."})).map_err(|e|e.to_string())?;
        if self.view.coverage.archive_bytes + bytes.len() > coverage::MAX_ARCHIVE_BYTES {
            return Err("The evidence budget is exhausted.".into());
        }
        write_evidence(
            &self
                .root_directory
                .join(format!("change-search-{number:02}-start.json")),
            &bytes,
        )?;
        self.view.coverage.archive_bytes += bytes.len();
        self.replace_memory_batch(plan, eligible, None)?;
        self.view.change_searches.push(summary);
        self.view.phase="Following this number across more memory. Let it change while four moments are collected.".into();
        Ok(())
    }

    pub fn finish_change_search(
        &mut self,
        investigation: &str,
        batch: usize,
        store: &crate::changes::Store,
    ) -> Result<()> {
        self.check_batch_transition(investigation, batch)?;
        self.changes_restored(store)?;
        if !self.collecting_change_search() {
            return Err("There is no collecting change search in this batch.".into());
        }
        if self.view.sample_count != search::MOMENTS {
            return Err("Collect exactly four moments before finishing this search.".into());
        }
        let index = self.view.change_searches.len() - 1;
        let mut summary = self.view.change_searches[index].clone();
        let selection = (|| {
            if self.view.frames.len() != search::MOMENTS
                || self.view.frames.iter().any(|f| {
                    f.width != summary.source_frame.width
                        || f.height != summary.source_frame.height
                        || f.epoch != self.view.frames[0].epoch
                })
            {
                return Err("An image is missing or the window changed size; the old number region cannot guide selection.".into());
            }
            summary.basis = self.view.frames.clone();
            let tracks: Vec<_> = self
                .view
                .tracks
                .iter()
                .filter(|t| t.active && same_text_region(t.bounds, summary.bounds))
                .collect();
            if tracks.len() != 1 {
                return Err("The selected number region is missing or ambiguous.".into());
            }
            if tracks[0].readings.len() != search::MOMENTS
                || tracks[0].readings.iter().any(|r| {
                    r.as_ref()
                        .is_none_or(|r| !same_text_region(r.bounds, summary.bounds))
                })
            {
                return Err(
                    "The number did not remain in the selected region across all four moments."
                        .into(),
                );
            }
            summary.observations = tracks[0]
                .samples
                .iter()
                .copied()
                .collect::<Option<Vec<_>>>()
                .ok_or("All four observations must read the selected number.")?;
            search::select(
                &self.plan,
                &self.snapshots,
                &self.invalid_chunks,
                summary.clone(),
                crate::value_search::MAX_WINDOWS
                    .min(self.budget_bytes / crate::value_search::WINDOW_BYTES),
            )
        })();
        let outcome = match selection {
            Ok(outcome) => outcome,
            Err(error) => {
                summary.status = "no_selection".into();
                summary.reason = Some(error.clone());
                self.view.change_searches[index] = summary;
                self.view.phase = error.clone();
                self.persist_change_search(index, None)?;
                return Err(error);
            }
        };
        self.view.change_searches[index] = outcome.summary.clone();
        self.persist_change_search(index, Some(&outcome))?;
        if outcome.windows.is_empty() {
            return Err(
                "No areas followed this number. Current observations were preserved.".into(),
            );
        }
        let count = outcome.windows.len();
        self.replace_memory_batch(outcome.windows, self.view.eligible_bytes, None)?;
        self.view.change_searches[index].status = "completed".into();
        self.view.phase = format!(
            "Found {count} areas following this number. Watching fresh changes before treating any as a finding."
        );
        Ok(())
    }

    fn persist_change_search(
        &mut self,
        index: usize,
        selection: Option<&search::Selection>,
    ) -> Result<()> {
        let summary = &self.view.change_searches[index];
        let bytes=serde_json::to_vec(&serde_json::json!({"schema_version":1,"investigation_id":self.view.investigation_id,
            "summary":summary,"windows":selection.map(|s|&s.windows),"witnesses":selection.map(|s|&s.witnesses),
            "limitation":"Four selection observations only. Natural-alignment little-endian i32/f32/f64 affine comparisons; no inferred gameplay meaning or write authority. Missing, changed and unvisited areas remain unknown. Fit and check wholly fresh observations."})).map_err(|e|e.to_string())?;
        if self.view.coverage.archive_bytes + bytes.len() > coverage::MAX_ARCHIVE_BYTES {
            return Err(
                "The evidence budget is exhausted; current observations were preserved.".into(),
            );
        }
        write_evidence(Path::new(&summary.evidence), &bytes)?;
        self.view.coverage.archive_bytes += bytes.len();
        Ok(())
    }
}
