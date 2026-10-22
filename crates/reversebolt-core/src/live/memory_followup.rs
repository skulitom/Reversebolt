use super::*;
use crate::{memory_followup as f, memory_trajectories as m};

impl LiveSession {
    /// Pin a report's complete selected groups before releasing the source batch.
    /// Selection never reads fresh bytes or uses a screen label. Only future
    /// captures touch the selected windows; original mappings remain unverified.
    pub fn prepare_memory_followup(
        &mut self,
        root: &Path,
        report: &m::Report,
        report_hash: &str,
        groups: &[usize],
        changes: &crate::changes::Store,
    ) -> Result<f::Reference> {
        // Reject stale requests before any target access.
        if report.investigation_id != self.view.investigation_id
            || report.batch != self.view.coverage.batch
            || report.capture_count != self.view.sample_count
            || self.view.memory_followup.is_some()
            || self
                .view
                .memory_proposals
                .as_ref()
                .is_none_or(|s| s.report_sha256 != report_hash)
            || visual::digest(&serde_json::to_vec(report).map_err(|e| e.to_string())?)
                != report_hash
        {
            return Err("Pin the current completed proposal report and exact capture count".into());
        }
        if groups.is_empty()
            || groups.len() > f::MAX_GROUPS
            || groups.iter().copied().collect::<BTreeSet<_>>().len() != groups.len()
            || groups.iter().any(|i| *i >= report.groups.len())
        {
            return Err("Select one to eight distinct existing proposal groups".into());
        }
        if changes.records.iter().any(|c| {
            c.scope.investigation == self.view.investigation_id
                && (c.phase.unresolved() || c.phase == crate::changes::Phase::Written)
        }) {
            return Err("Restore the applied trial before starting a memory follow-up".into());
        }
        let mut windows = BTreeMap::new();
        let mut candidates = vec![];
        let mut sorted = groups.to_vec();
        sorted.sort_unstable();
        for group in sorted {
            for (member, c) in report.groups[group].members.iter().enumerate() {
                let original = self
                    .plan
                    .iter()
                    .find(|p| p.address == c.chunk_address)
                    .ok_or("Proposal source is outside this memory batch")?;
                let address = c.selection.address;
                let end = original
                    .address
                    .checked_add(original.length as u64)
                    .ok_or("Invalid source range")?;
                if address < original.address
                    || address
                        .checked_add(c.selection.kind.width() as u64)
                        .is_none_or(|n| n > end)
                {
                    return Err("Proposal crosses its original read window".into());
                }
                // Original plans use aligned regions; clipping the last tile
                // keeps every byte inside its original mapping and read chunk.
                let start = (address & !63).max(original.address);
                let length = (end.min((address & !63).saturating_add(64)) - start) as usize;
                windows.entry(start).or_insert(Chunk {
                    address: start,
                    length,
                    region: original.region.clone(),
                });
                candidates.push(f::Candidate {
                    group,
                    member,
                    selection: c.selection.clone(),
                    source_verdict: c.assessment.verdict,
                    source_mapping_invalidated: c.chunk_invalidated_in_recording,
                    window: start,
                });
            }
        }
        let plan = f::Plan {
            policy: f::POLICY.into(),
            investigation_id: report.investigation_id.clone(),
            source_batch: report.batch,
            source_captures: report.capture_count,
            source_report_sha256: report_hash.into(),
            source_manifest_sha256: report.manifest_sha256.clone(),
            batch: report.batch + 1,
            process_created: self.view.process.created.clone(),
            window: self.view.process.window,
            candidates,
            windows: windows.into_values().collect(),
        };
        plan.validate()?;
        self.check_batch_transition(&report.investigation_id, report.batch)?;
        // Publication precedes the transition and every future sample. Failed
        // transitions leave an immutable reservation; only exact retry can reuse it.
        let reference = f::save_plan(root, &plan)?;
        self.replace_memory_batch(plan.windows, self.view.eligible_bytes, None)?;
        self.view.memory_followup = Some(reference.clone());
        self.view.phase = "Watching fresh changes in selected memory patterns.".into();
        Ok(reference)
    }
}
