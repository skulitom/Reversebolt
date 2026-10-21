use super::{tests::*, *};
fn view() -> EngineView {
    let mut view = super::runtime_watch_tests::runtime_view();
    let live = view.live.as_ref().unwrap();
    let c = &live.runtime.cards[0].candidate;
    view.freezes.push(reversebolt_core::freezes::Freeze {
        id: uuid::Uuid::new_v4().to_string(),
        actor: "mcp-agent".into(),
        investigation: live.investigation_id.clone(),
        catalog: live.runtime.catalog_id.clone(),
        requested_revision: 0,
        candidate: c.id.clone(),
        value: "3".into(),
        kind: c.type_code,
        path: c.locator.clone(),
        identity: None,
        phase: "active".into(),
        reason: String::new(),
        checks: 0,
        writes: 0,
        checked_ms: 0,
        intent: None,
        evidence: None,
    });
    view
}
#[test]
fn stop_queued_during_a_check_prevents_all_later_checks() {
    let (sent, received) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    let mut gate = Some(wait);
    let worker = EngineHandle::test_worker(view(), move |command, view| {
        sent.send(command.action.clone()).unwrap();
        if command.action == "_tick_runtime_freeze" {
            gate.take().unwrap().recv().unwrap();
            view.freezes[0].checked_ms = reversebolt_core::changes::now_ms();
        } else {
            assert_eq!(command.action, "set_runtime_freeze");
            view.freezes[0].stop("off");
        }
        json!({"ok":true,"state":view})
    });
    let mut c = controller(worker);
    let id = c.engine.view.read().unwrap().freezes[0].id.clone();
    c.tick_freezes();
    assert_eq!(
        received.recv_timeout(Duration::from_secs(2)).unwrap(),
        "_tick_runtime_freeze"
    );
    let (send, reply) = mpsc::channel();
    c.command((
        Command {
            action: "set_runtime_freeze".into(),
            source: "desktop".into(),
            args: json!({"freeze_id":id,"enabled":false}),
        },
        Some(send),
    ));
    assert!(reply.try_recv().is_err());
    release.send(()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        c.finish_manual();
        c.tick_freezes();
        if let Ok(r) = reply.try_recv() {
            assert_eq!(r["ok"], true);
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        received.recv_timeout(Duration::from_secs(1)).unwrap(),
        "set_runtime_freeze"
    );
    for _ in 0..4 {
        c.tick_freezes();
    }
    assert!(received.try_recv().is_err());
}
#[test]
fn private_tick_cannot_be_requested_through_public_commands() {
    let mut c = controller(EngineHandle::test_worker(view(), |_, _| {
        panic!("No worker command expected")
    }));
    let (send, reply) = mpsc::channel();
    c.command((
        Command {
            action: "_tick_runtime_freeze".into(),
            source: "desktop".into(),
            args: json!({}),
        },
        Some(send),
    ));
    assert_eq!(reply.recv().unwrap()["ok"], false);
}
