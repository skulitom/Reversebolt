//! Pin existing questions before an input experiment; this is not fresh discovery.
use crate::{Result, runtime, runtime_hypotheses::Review, visual};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Binding {
    pub catalog_id: String,
    pub revision: u64,
    pub review_id: String,
    pub review_sha256: String,
    pub report_sha256: String,
}
impl Binding {
    pub fn capture(view: &runtime::View) -> Result<Option<Self>> {
        let Some(review) = &view.naming else {
            return Ok(None);
        };
        if review.status != "complete"
            || review.error.is_some()
            || review
                .batch
                .as_ref()
                .is_none_or(|b| b.accepted.is_empty() || b.report_sha256 != review.report_sha256)
            || view
                .catalog_source
                .as_ref()
                .is_none_or(|s| s.document_sha256 != review.report_sha256)
        {
            return Ok(None);
        }
        Ok(Some(Self {
            catalog_id: view.catalog_id.clone(),
            revision: view.revision,
            review_id: review.evidence_id.clone(),
            report_sha256: review.report_sha256.clone(),
            review_sha256: visual::digest(&serde_json::to_vec(review).map_err(|e| e.to_string())?),
        }))
    }
    pub fn resolve(&self, view: &runtime::View) -> Result<Review> {
        let current =
            Self::capture(view)?.ok_or("The pinned metadata questions are no longer available")?;
        if current.catalog_id != self.catalog_id
            || current.revision != self.revision
            || current.review_id != self.review_id
            || current.review_sha256 != self.review_sha256
            || current.report_sha256 != self.report_sha256
        {
            return Err(
                "The metadata catalogue or questions changed during the input experiment".into(),
            );
        }
        Ok(view.naming.clone().unwrap())
    }
}
