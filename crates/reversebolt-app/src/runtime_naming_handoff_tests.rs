//! Exercises the real submission/archive path with a target that forbids all I/O.
use super::*;
use reversebolt_core::{live::*, play_context as pc, runtime_hypotheses as h};

struct NoTargetAccess(ProcessInfo);
impl NativeTarget for NoTargetAccess {
    fn info(&self) -> &ProcessInfo {
        &self.0
    }
    fn regions(&self) -> Result<Vec<Region>> {
        Ok(vec![Region {
            base: 0x1000,
            length: 4096,
            ..Default::default()
        }])
    }
    fn read(&self, _: u64, _: usize) -> Result<Vec<u8>> {
        panic!("hypothesis submission must not read a target")
    }
    fn screen(&self, _: &Path) -> Result<Screen> {
        panic!("hypothesis submission must not capture")
    }
    fn alive(&self) -> bool {
        true
    }
}

#[test]
fn agent_submission_keeps_activity_in_durable_receipts_without_republishing_old_reviews() {
    let parent = std::env::temp_dir().canonicalize().unwrap();
    let directory = parent.join(format!("reversebolt-handoff-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&directory).unwrap();
    let key = |name| json!({"image":"Fixture.dll","type_name":name,"layout_sha256":"a".repeat(64)});
    let report = json!({"schema":4,"pid":77,"created":"0123456789abcdef","session":2,"edit_authority":false,
        "classes":[
            {"id":0,"key":key("Roots"),"shape":"object","fields":[{"id":0,"name":"Current","reference":true,"type_code":18,"width":8,"flags":16}]},
            {"id":1,"key":key("Actor"),"shape":"object","fields":[{"id":0,"name":"Speed","reference":false,"type_code":8,"width":4,"flags":0}]}],
        "snapshots":[{"snapshot_id":0,"nodes":[{"id":0,"class":1,"address":"0x1000"}],
            "roots":[{"class":0,"field":0,"read_ok":true,"node":{"id":0}}],
            "reads":[{"node":0,"declaring_class":1,"field":0,"element_index":null,"type_code":8,"bytes":8i32.to_le_bytes(),"value":8,"reference":null,"read_ok":true}]}]});
    let report_path = directory.join("report.json");
    save_new(&report_path, &report).unwrap();
    let inventory = reversebolt_core::runtime_inventory::build(&report).unwrap();
    let selected_types: Vec<_> = inventory.types.iter().map(|t| t.id).collect();
    let selection = h::select(&inventory, &selected_types).unwrap();
    assert_eq!(selection.rows.len(), 1);
    let hypothesis = h::Proposal {
        candidate: selection.rows[0].id,
        name: "Speed".into(),
        explanation: "The label suggests speed; its gameplay effect is unverified.".into(),
        citations: vec![h::Citation {
            source: "field_name".into(),
            index: None,
            quote: "Speed".into(),
        }],
        alternatives: vec![],
        question: "Does this value affect the observed response?".into(),
    };
    let mut engine = Engine::load(directory.join("state.json")).unwrap();
    let info = ProcessInfo {
        pid: 77,
        created: "0123456789abcdef".into(),
        session: 2,
        window: 1,
        ..Default::default()
    };
    let mut live = LiveSession::attach(
        Box::new(NoTargetAccess(info)),
        1,
        directory.join("captures"),
    )
    .unwrap();
    live.view.frames = (1..=6).map(|number| serde_json::from_value(json!({
        "id":format!("frame-{number}"),"number":number,"sha256":"b".repeat(64),"width":10,"height":10,
        "epoch":0,"started_unix_ms":number,"capture_interval":{"clock_id":live.view.investigation_id,
            "begin_us":number * 1_000_000,"end_us":number * 1_000_000 + 1000}})).unwrap()).collect();
    let frame = |i: usize| {
        let f = &live.view.frames[i];
        pc::Frame {
            id: f.id.clone(),
            sha256: f.sha256.clone(),
            number: f.number,
            width: f.width,
            height: f.height,
            interval: f.capture_interval.clone().unwrap(),
        }
    };
    let mut context = pc::Context::from_observation(
        pc::Source {
            investigation_id: live.view.investigation_id.clone(),
            batch: 1,
            frame_metadata_sha256: reversebolt_core::visual::digest(
                &serde_json::to_vec(&live.view.frames).unwrap(),
            ),
            key: 68,
            pairs: [
                pc::Pair {
                    episode: 0,
                    before: frame(0),
                    after: frame(1),
                    input_sha256: "c".repeat(64),
                    difference: 12.,
                },
                pc::Pair {
                    episode: 2,
                    before: frame(3),
                    after: frame(4),
                    input_sha256: "c".repeat(64),
                    difference: 12.,
                },
            ],
            quiet_difference: 1.,
        },
        pc::Observation {
            consistent_visible_change: false,
            description: "Ambiguous visible change.".into(),
            uncertainty: "Animation could explain it.".into(),
        },
    )
    .unwrap();
    use reversebolt_core::activity_motion as motion;
    context.motion = Some(motion::Summary {
        metric: motion::METRIC_V2.into(),
        frame_metadata_sha256: context.source.frame_metadata_sha256.clone(),
        measurement_dimensions: [256, 144],
        maximum_selected: 48,
        grid: [6, 4],
        pairs: context.source.pairs.clone().map(|p| motion::Pair {
            before_id: p.before.id,
            before_sha256: p.before.sha256,
            after_id: p.after.id,
            after_sha256: p.after.sha256,
            measurement: Some(motion::Measurement {
                selected: 2,
                matched_cells: 1,
                unresolved: [("poor_match".into(), 1)].into(),
                matches: vec![motion::Match {
                    at: [20, 20],
                    delta: [18, 0],
                }],
                consensus: None,
            }),
            error: None,
        }),
    });
    context.validate().unwrap();
    let original = Review {
        evidence_id: uuid::Uuid::new_v4().to_string(),
        model: "original-model".into(),
        status: "complete".into(),
        report_sha256: inventory.report_sha256.clone(),
        play_context: Some(context.clone()),
        question_source: Default::default(),
        question_policy: None,
        batch: Some(h::validate(&selection, h::Proposals { hypotheses: vec![] }).unwrap()),
        error: None,
    };
    live.view.runtime = runtime::View {
        catalog_id: uuid::Uuid::new_v4().to_string(),
        revision: 4,
        catalog_source: Some(runtime::CatalogSource {
            path: report_path.to_string_lossy().into_owned(),
            document_sha256: inventory.report_sha256.clone(),
            observed_unix_ms: 0,
        }),
        naming: Some(original.clone()),
        ..Default::default()
    };
    let catalog = live.view.runtime.catalog_id.clone();
    engine.live = Some(live);
    let request = |after: &str| reversebolt_core::Command {
        action: "submit_runtime_hypotheses".into(),
        source: "fixture-agent".into(),
        args: json!({"review_id":uuid::Uuid::new_v4().to_string(),"catalog_id":catalog,"after_revision":4,
            "after_review":after,"report_sha256":inventory.report_sha256,"selected_types":selected_types,"hypotheses":[hypothesis]}),
    };
    let first_request = request(&original.evidence_id);
    let base = directory.join("runtime");
    let first = command(&mut engine, &first_request, &base).unwrap();
    assert_eq!(first["activity_transfer"]["retained"], true);
    assert_eq!(first["review"]["model"], "agent:fixture-agent");
    let first_context = &first["review"]["play_context"];
    assert_eq!(first_context["observation"], json!(context.observation));
    assert_eq!(first_context["motion"], json!(context.motion));
    assert_eq!(
        first_context["interpretation_source"],
        json!({"review_id":original.evidence_id,"model":"original-model"})
    );
    assert!(engine.live.as_ref().unwrap().view.runtime.cards.is_empty());
    let second_request = request(first_request.args["review_id"].as_str().unwrap());
    let second = command(&mut engine, &second_request, &base).unwrap();
    assert_eq!(second["review"]["play_context"], *first_context);
    let replay = command(&mut engine, &first_request, &base).unwrap();
    assert_eq!(replay["review"], first["review"]);
    assert_eq!(replay["activity_transfer"], first["activity_transfer"]);
    assert_eq!(replay["replayed"], true);
    assert_eq!(
        engine
            .live
            .as_ref()
            .unwrap()
            .view
            .runtime
            .naming
            .as_ref()
            .unwrap()
            .evidence_id,
        second_request.args["review_id"].as_str().unwrap()
    );
    let mut changed = first_request.clone();
    changed.args["hypotheses"][0]["question"] =
        json!("Does this changed request have a different effect?");
    assert!(command(&mut engine, &changed, &base).is_err());
    let stale_request = request(&original.evidence_id);
    assert!(command(&mut engine, &stale_request, &base).is_err());
    // Changed observations still allow new questions, but carry no stale activity.
    engine.live.as_mut().unwrap().view.frames[0].sha256 = "d".repeat(64);
    let third_request = request(second_request.args["review_id"].as_str().unwrap());
    let third = command(&mut engine, &third_request, &base).unwrap();
    assert_eq!(third["activity_transfer"]["retained"], false);
    assert!(third["review"]["play_context"].is_null());
    drop(engine);
    let mut reopened = Engine::load(directory.join("state.json")).unwrap();
    assert!(reopened.live.is_none());
    let replay = command(&mut reopened, &first_request, &base).unwrap();
    assert_eq!(replay["review"], first["review"]);
    assert_eq!(replay["activity_transfer"], first["activity_transfer"]);
    drop(reopened);
    let resolved = directory.canonicalize().unwrap();
    assert_eq!(resolved.parent(), Some(parent.as_path()));
    assert!(
        resolved
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("reversebolt-handoff-")
    );
    std::fs::remove_dir_all(resolved).unwrap();
}
