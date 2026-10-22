//! Reuse image interpretations independently of the author of metadata questions.
use super::*;
use reversebolt_core::{play_context::Context, visual::FrameRef};
use reversebolt_observer::{runtime_activity as a, runtime_naming as n};
use std::sync::atomic::AtomicBool;

pub(crate) struct Retained {
    pub receipt: a::Receipt,
    pub context: Context,
    pub provenance: Value,
}

fn retained(
    engine: &Engine,
    base: &Path,
    summary: &reversebolt_core::activity_experiment::Review,
) -> Result<Retained> {
    let live = &engine.live.as_ref().ok_or("Application disconnected")?.view;
    let archive =
        crate::activity_experiment::review::read_result(engine, &summary.experiment, base)?;
    let result = &archive["result"];
    if summary.status != "interpreted"
        || summary.error.is_some()
        || result["status"] != "interpreted"
        || result["cancelled"] != false
        || !result["error"].is_null()
        || result["model"] != json!(summary.model)
    {
        return Err("The saved image interpretation is incomplete or cancelled".into());
    }
    let frames = summary
        .frame_ids
        .iter()
        .map(|id| {
            live.frames
                .iter()
                .find(|f| &f.id == id)
                .cloned()
                .ok_or("A reviewed input image is missing".to_string())
        })
        .collect::<Result<Vec<_>>>()?;
    let receipt: a::Receipt =
        serde_json::from_value(result["interpretation"].clone()).map_err(|e| e.to_string())?;
    let context = receipt
        .context()
        .ok_or("The saved interpretation has no valid context")?
        .bind_frames(&frames)?
        .bind_interpretation(
            result["review_id"]
                .as_str()
                .ok_or("Missing original review identity")?,
            summary
                .model
                .as_deref()
                .ok_or("Missing original image model")?,
        )?;
    let published = summary
        .context
        .clone()
        .ok_or("Missing published input interpretation")?
        .bind_frames(&frames)?;
    if context != published
        || summary.source.as_ref() != Some(&context.source)
        || context.source.investigation_id != live.investigation_id
        || context.source.batch != live.coverage.batch
    {
        return Err(
            "The retained interpretation differs from its original evidence or scope".into(),
        );
    }
    context.resolve_frames(&live.frames)?;
    Ok(Retained {
        receipt,
        context,
        provenance: json!({"reused":true,"experiment":summary.experiment,
        "receipt_sha256":summary.receipt_sha256,"interpretation_source":published.interpretation_source,
        "new_image_model_calls":0,"limitation":"Retained interpretation, not independent validation of the new metadata hypotheses."}),
    })
}

pub(crate) fn latest(engine: &Engine, base: &Path) -> Result<Option<Retained>> {
    let Some(session) = &engine.live else {
        return Ok(None);
    };
    // Do not skip a newer unavailable/cancelled experiment to resurrect an older
    // interpretation. The current result remains explicit.
    let Some(summary) = session.view.input_experiments.last() else {
        return Ok(None);
    };
    if summary.status != "interpreted" || summary.error.is_some() {
        return Ok(None);
    }
    retained(engine, base, summary).map(Some)
}

pub(crate) struct Prepared {
    pub input: Option<a::Input>,
    pub frames: Vec<FrameRef>,
    pub retained: Option<Retained>,
    pub reuse_error: Option<String>,
}
pub(crate) struct Outcome {
    pub receipt: n::Receipt,
    pub context: Option<Context>,
    pub reuse: Value,
}
pub(crate) fn prepare(engine: &Engine, base: &Path, directory: &Path) -> Result<Prepared> {
    let input = prepare_activity(
        engine.live.as_ref().ok_or("Application disconnected")?,
        directory,
    )?;
    let archive = read_archive(&directory.join("play-source.json"))?;
    let frames = if input.is_some() {
        serde_json::from_value(archive["frames"].clone()).map_err(|e| e.to_string())?
    } else {
        vec![]
    };
    let has_experiment = engine
        .live
        .as_ref()
        .is_some_and(|s| !s.view.input_experiments.is_empty());
    let (retained, reuse_error) = match latest(engine, base) {
        Ok(Some(retained))
            if input.as_ref().is_some_and(|input| {
                retained.context.source == input.source
                    && retained.context.regions == input.regions
                    && retained.context.appearances == input.appearances
            }) =>
        {
            (Some(retained), None)
        }
        Ok(Some(_)) => (
            None,
            Some("Current measured activity differs from its retained interpretation".into()),
        ),
        Ok(None) if has_experiment => (
            None,
            Some("The latest input experiment has no completed interpretation to reuse".into()),
        ),
        Ok(None) => (None, None),
        Err(error) => (None, Some(error)),
    };
    Ok(Prepared {
        input,
        frames,
        retained,
        reuse_error,
    })
}
impl Prepared {
    pub fn infer(
        self,
        id: &str,
        model: &str,
        cancel: &AtomicBool,
        execute: impl FnOnce(Option<&a::Input>) -> Result<n::Receipt>,
    ) -> Result<Outcome> {
        // Metadata questions are still generated independently. Supplying no
        // image input prevents that pass from reinterpreting a retained clip.
        let mut receipt = execute(if self.retained.is_some() || self.reuse_error.is_some() {
            None
        } else {
            self.input.as_ref()
        })?;
        let mut context = None;
        let mut reuse = if let Some(error) = &self.reuse_error {
            json!({"reused":false,"error":error,"new_image_model_calls":0})
        } else {
            json!({"reused":false})
        };
        if !cancel.load(Ordering::SeqCst) && !receipt.cancelled && receipt.error.is_none() {
            if let Some(retained) = self.retained {
                if receipt.activity.is_some() {
                    return Err(
                        "Metadata-only inference unexpectedly returned an image interpretation"
                            .into(),
                    );
                }
                receipt.activity = Some(retained.receipt);
                context = Some(retained.context);
                reuse = retained.provenance;
            } else if self.reuse_error.is_some() {
                if receipt.activity.is_some() {
                    return Err(
                        "Metadata-only inference unexpectedly returned an image interpretation"
                            .into(),
                    );
                }
            } else {
                if let Some(activity) = &receipt.activity
                    && self.input.as_ref().is_none_or(|i| {
                        i.source != activity.source
                            || i.regions != activity.regions
                            || i.appearances != activity.appearances
                    })
                {
                    return Err("Fresh image interpretation differs from its measured input".into());
                }
                let bound = receipt
                    .activity
                    .as_ref()
                    .and_then(|a| a.context())
                    .map(|c| c.bind_frames(&self.frames)?.bind_interpretation(id, model))
                    .transpose();
                match bound {
                    Ok(value) => context = value,
                    Err(error) => {
                        reuse = json!({"reused":false,"error":error,"limitation":"Image context unavailable; independently checked metadata questions are retained."})
                    }
                }
            }
        }
        if cancel.load(Ordering::SeqCst) {
            receipt.cancelled = true;
            receipt.batch = None;
            context = None;
        }
        Ok(Outcome {
            receipt,
            context,
            reuse,
        })
    }
}
