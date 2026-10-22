use super::*;
use crate::inputs::{Buffer, Poll};

fn interval(begin: u64, end: u64) -> CaptureInterval {
    CaptureInterval {
        clock_id: "investigation".into(),
        begin_us: begin,
        end_us: end,
    }
}
fn context(key: Option<u8>) -> (CaptureInterval, CaptureInterval, Window, Window) {
    let a = interval(1_000_000, 1_010_000);
    let b = interval(2_000_000, 2_010_000);
    let mut buffer = Buffer::new("investigation".into(), "sampler".into(), 0);
    for t in (1_000_000..2_010_000).step_by(20_000) {
        buffer
            .push(Poll {
                begin_us: t,
                end_us: t + 100,
                focus: Focus::SelectedForeground,
                down: key.into_iter().collect(),
            })
            .unwrap();
    }
    let old = buffer.window(a.begin_us, a.end_us).unwrap();
    let new = buffer.window(a.begin_us, b.end_us).unwrap();
    (a, b, old, new)
}

#[test]
fn memory_input_uses_memory_bounds_and_records_held_keys_without_claiming_delivery() {
    for key in [None, Some(65)] {
        let (a, b, old, w) = context(key);
        assert_eq!(
            classify(&a, &b, &old, &w, "investigation"),
            Ok(key.map_or(Exposure::NoKeysDetected, Exposure::Key))
        );
    }
    let (a, b, old, mut w) = context(Some(65));
    // An image-only window ends before this later memory operation. Quiet pixels
    // or a plausible polling summary cannot supply that missing memory coverage.
    w.requested_end_us = b.begin_us - 1;
    assert_eq!(
        classify(&a, &b, &old, &w, "investigation"),
        Err(Unknown::Coverage)
    );
}

#[test]
fn memory_input_abstains_on_scope_gaps_ambiguous_keys_and_malformed_polls() {
    let (a, b, old, w) = context(Some(65));
    for change in 0..12 {
        let (mut a, mut b, mut old, mut w) = (a.clone(), b.clone(), old.clone(), w.clone());
        let reason = match change {
            0 => {
                a.clock_id = "other".into();
                Unknown::Timing
            }
            1 => {
                b.begin_us = a.end_us;
                Unknown::Timing
            }
            2 => {
                old.generation = "restarted".into();
                Unknown::Generation
            }
            3 => {
                w.monitor_started_us = a.begin_us + 1;
                Unknown::Coverage
            }
            4 => {
                w.retained_from_us = a.begin_us + 1;
                Unknown::Coverage
            }
            5 => {
                w.omitted_polls = 1;
                Unknown::Coverage
            }
            6 => {
                w.polls[4].focus = Focus::OutsideSelected;
                Unknown::Focus
            }
            7 => {
                w.polls.drain(5..15);
                w.largest_unsampled_us = 0;
                Unknown::Gap
            }
            8 => {
                w.polls[3].down = vec![65, 66];
                Unknown::MixedKeys
            }
            9 => {
                for p in &mut w.polls {
                    p.down.clear();
                }
                w.polls[4].down = vec![65];
                Unknown::OnePressedPoll
            }
            10 => {
                w.polls[4] = w.polls[3].clone();
                Unknown::MalformedPolls
            }
            _ => {
                w.polls[4].down = vec![255];
                Unknown::MalformedPolls
            }
        };
        assert_eq!(
            classify(&a, &b, &old, &w, "investigation"),
            Err(reason),
            "case {change}"
        );
    }
    let (mut a, mut b, old, mut w) = context(None);
    a.begin_us = 1_000_000;
    a.end_us = 1_000_001;
    b.begin_us = 1_000_010;
    b.end_us = 1_000_030;
    w.requested_end_us = b.end_us;
    w.polls = vec![
        Poll {
            begin_us: a.begin_us,
            end_us: a.begin_us + 1,
            focus: Focus::SelectedForeground,
            down: vec![],
        },
        Poll {
            begin_us: b.begin_us,
            end_us: b.begin_us + 1,
            focus: Focus::SelectedForeground,
            down: vec![],
        },
    ];
    assert_eq!(
        classify(&a, &b, &old, &w, "investigation"),
        Err(Unknown::TooFewPolls)
    );
}

#[test]
fn memory_input_keeps_omitted_key_classes_and_fits_the_existing_report_budget() {
    use crate::{
        memory_followup as f,
        memory_trajectories::{Assessment, Verdict},
    };
    let values: Vec<_> = (0..20)
        .map(|i| Some(999_999_999_990.125 + i as f64 * 0.125))
        .collect();
    let mut sizes = vec![];
    for known in [0, 8, 19] {
        let intervals = (0..19)
            .map(|i| {
                if i < known {
                    Exposure::Key(i as u8 + 1)
                } else {
                    Exposure::Unknown(Unknown::MalformedPolls)
                }
            })
            .collect();
        let evidence = Evidence {
            intervals,
            no_keys_detected: Counts::default(),
            keys: (1..=known.min(8))
                .map(|key| KeyCounts {
                    key: key as u8,
                    counts: Counts {
                        increased: 1,
                        ..Counts::default()
                    },
                })
                .collect(),
            omitted_keys: (9..=known).map(|k| k as u8).collect(),
            unknown_pairs: 19 - known,
        };
        assert!(evidence.valid(&values));
        let mut changed = evidence.clone();
        changed.unknown_pairs += 1;
        assert!(!changed.valid(&values));
        if !evidence.keys.is_empty() {
            let mut changed = evidence.clone();
            changed.keys[0].counts.increased += 1;
            assert!(!changed.valid(&values));
        }
        let outcomes = (0..f::MAX_MEMBERS)
            .map(|i| f::Outcome {
                group: i / 12,
                member: i % 12,
                assessment: Assessment {
                    values: values.clone(),
                    missing_reads: 0,
                    invalid_numbers: 0,
                    checked: 19,
                    violations: 0,
                    verdict: Verdict::Consistent,
                },
                changed_pairs: 19,
                mapping_invalidated_in_recording: false,
                input: Some(evidence.clone()),
            })
            .collect();
        let report = f::Report {
            reference: f::Reference {
                investigation_id: uuid::Uuid::new_v4().to_string(),
                batch: 24,
                plan_sha256: "a".repeat(64),
            },
            evidence_manifest_sha256: "b".repeat(64),
            captures: 48,
            memory_captures: (29..=48).collect(),
            outcomes,
            limitation: f::LIMITATION.into(),
            input_associations: Some(Description::default()),
        };
        let bytes = serde_json::to_vec(&report).unwrap();
        assert!(bytes.len() < f::MAX_FILE_BYTES);
        sizes.push(bytes.len());
    }
    println!("MEMORY_INPUT_REPORT_BYTES={sizes:?}");
}
