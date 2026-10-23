//! Request accounting independent of whether the local server completes.
use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IncompleteStage {
    pub stage: Stage,
    pub phase: String,
    pub error: String,
}

/// Bytes provide a conservative bound for small text inputs. Large requests
/// retain their existing context and all requests retain the post-response check.
pub(super) fn context_for(bytes: usize, output: usize, requested: usize) -> usize {
    if requested > 16384
        && bytes
            .checked_add(output)
            .and_then(|n| n.checked_add(512))
            .is_some_and(|n| n <= 16384)
    {
        16384
    } else {
        requested
    }
}

pub(super) fn finish(
    receipt: &mut Receipt,
    stage: Stage,
    failure: Option<(String, String)>,
) -> Result<usize> {
    if let Some((phase, error)) = failure {
        receipt.incomplete_stage = Some(IncompleteStage {
            stage,
            phase,
            error: error.clone(),
        });
        receipt.error = Some(error.clone());
        return Err(error);
    }
    let index = receipt.stages.len();
    receipt.stages.push(stage);
    Ok(index)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn receipt() -> Receipt {
        serde_json::from_value(json!({"schema":1,"report_sha256":"a".repeat(64),
            "model":"fixture","model_digest":"b".repeat(64),"server_version":"fixture",
            "stages":[],"selection":null,"batch":null,"error":null,"cancelled":false,"limitation":"fixture"})).unwrap()
    }
    fn stage() -> Stage {
        let prompt = "Observed labels, unchanged.".to_owned();
        Stage {
            prompt_sha256: digest(prompt.as_bytes()),
            prompt,
            schema: json!({"type":"object"}),
            options: json!({"temperature":0,"seed":42,"num_ctx":16384,"num_predict":1700}),
            response: Value::Null,
            elapsed_ms: 90000,
        }
    }
    #[test]
    fn small_requests_keep_output_headroom_without_shrinking_large_inputs() {
        assert_eq!(context_for(16384 - 1700 - 512, 1700, 32768), 16384);
        assert_eq!(context_for(16384 - 1700 - 512 + 1, 1700, 32768), 32768);
        assert_eq!(context_for(12000, 600, 49152), 16384);
        assert_eq!(context_for(800, 600, 8192), 8192);
        assert_eq!(context_for(usize::MAX, 600, 32768), 32768);
        assert_eq!(context_for(800, usize::MAX, 32768), 32768);
        assert_eq!(context_for("界".repeat(5000).len(), 1700, 32768), 32768);
    }
    #[test]
    fn failed_request_retains_exact_evidence_and_completed_stages() {
        let mut receipt = receipt();
        let mut completed = stage();
        completed.response = json!({"done":true});
        finish(&mut receipt, completed.clone(), None).unwrap();
        let attempt = stage();
        let saved = serde_json::to_value(&attempt).unwrap();
        assert_eq!(
            finish(
                &mut receipt,
                attempt,
                Some(("request".into(), "Timed out".into()))
            )
            .unwrap_err(),
            "Timed out"
        );
        assert_eq!(receipt.error.as_deref(), Some("Timed out"));
        assert_eq!(
            serde_json::to_value(&receipt.stages[0]).unwrap(),
            serde_json::to_value(completed).unwrap()
        );
        assert_eq!(receipt.stages.len(), 1);
        let retained = receipt.incomplete_stage.as_ref().unwrap();
        assert_eq!(retained.phase, "request");
        assert_eq!(serde_json::to_value(&retained.stage).unwrap(), saved);
        let roundtrip: Receipt =
            serde_json::from_value(serde_json::to_value(&receipt).unwrap()).unwrap();
        assert_eq!(roundtrip.incomplete_stage.unwrap().error, "Timed out");
    }
    #[test]
    fn identity_failure_keeps_returned_text_and_legacy_receipts_stay_readable() {
        let mut receipt = receipt();
        assert!(receipt.incomplete_stage.is_none());
        assert!(
            serde_json::to_value(&receipt)
                .unwrap()
                .get("incomplete_stage")
                .is_none()
        );
        let mut attempt = stage();
        attempt.response = json!({"message":{"content":"untrusted response"}});
        assert!(
            finish(
                &mut receipt,
                attempt,
                Some(("identity_recheck".into(), "Model changed".into()))
            )
            .is_err()
        );
        assert!(receipt.stages.is_empty());
        assert_eq!(
            receipt.incomplete_stage.unwrap().stage.response["message"]["content"],
            "untrusted response"
        );
    }
}
