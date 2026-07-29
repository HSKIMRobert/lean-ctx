use super::{
    AgentRecord, AgentStatus, check, decommission, gc, get, heartbeat, register, resume, spiffe_id,
    suspend, suspend_agents_for_owner,
};
use std::collections::BTreeMap;

fn isolated() -> crate::core::data_dir::IsolatedDataDir {
    crate::core::data_dir::isolated_data_dir()
}

#[test]
fn owner_is_mandatory_and_role_must_exist() {
    let _iso = isolated();
    assert!(register("a1", "coder", " ").is_err());
    assert!(register("a1", "no-such-role", "yves@org").is_err());
    assert!(register("a/1", "coder", "yves@org").is_err());
}

#[test]
fn identity_registry_does_not_overwrite_mcp_presence_registry() {
    let iso = isolated();
    let agents_dir = iso.path().join("agents");
    std::fs::create_dir_all(&agents_dir).expect("agents dir");
    let presence_path = agents_dir.join("registry.json");
    let presence = r#"{"agents":[{"agent_id":"mcp-1"}]}"#;
    std::fs::write(&presence_path, presence).expect("presence registry");
    register("identity-1", "coder", "yves@org").expect("register identity");
    assert_eq!(
        std::fs::read_to_string(presence_path).expect("presence survives"),
        presence
    );
    assert!(agents_dir.join("identity-registry.json").exists());
}

#[test]
fn legacy_identity_registry_migrates_on_next_write() {
    let iso = isolated();
    let agents_dir = iso.path().join("agents");
    std::fs::create_dir_all(&agents_dir).expect("agents dir");
    let legacy_path = agents_dir.join("registry.json");
    let legacy = AgentRecord {
        agent_id: "legacy-1".to_string(),
        role: "coder".to_string(),
        owner: "yves@org".to_string(),
        status: AgentStatus::Active,
        created_at: String::new(),
        public_key: String::new(),
        attestation: None,
        last_heartbeat: None,
        suspended_reason: None,
        decommissioned_at: None,
        pid: None,
        last_seen: None,
    };
    let records = BTreeMap::from([(legacy.agent_id.clone(), legacy)]);
    std::fs::write(&legacy_path, serde_json::to_string(&records).expect("JSON"))
        .expect("legacy registry");
    assert!(get("legacy-1").is_some());
    heartbeat("legacy-1").expect("migrate heartbeat");
    assert!(agents_dir.join("identity-registry.json").exists());
}

#[test]
fn lifecycle_register_suspend_resume_decommission() {
    let _iso = isolated();
    let rec = register("agent-x", "coder", "yves@org").expect("register");
    assert_eq!(rec.status, AgentStatus::Active);
    assert_eq!(rec.public_key.len(), 64);
    assert!(rec.attestation.is_some());
    assert!(register("agent-x", "coder", "yves@org").is_err());
    assert!(check("agent-x").allowed);
    suspend("agent-x", "incident review").expect("suspend");
    assert!(!check("agent-x").allowed);
    resume("agent-x").expect("resume");
    assert!(check("agent-x").allowed);
    decommission("agent-x").expect("decommission");
    assert!(!check("agent-x").allowed);
    assert!(resume("agent-x").is_err());
    assert!(get("agent-x").expect("kept").decommissioned_at.is_some());
}

#[test]
fn owner_offboarding_suspends_only_their_active_agents() {
    let _iso = isolated();
    register("a-alice-1", "coder", "alice@org").expect("r1");
    register("a-alice-2", "reviewer", "alice@org").expect("r2");
    register("a-bob-1", "coder", "bob@org").expect("r3");
    decommission("a-alice-2").expect("gone");
    let hit = suspend_agents_for_owner("alice@org", "SCIM deactivated").expect("offboard");
    assert_eq!(hit, vec!["a-alice-1".to_string()]);
    assert_eq!(get("a-bob-1").expect("bob").status, AgentStatus::Active);
    assert_eq!(
        get("a-alice-1").expect("alice").suspended_reason.as_deref(),
        Some("SCIM deactivated")
    );
}

#[test]
fn unregistered_agents_are_flagged() {
    let _iso = isolated();
    let check = check("ghost");
    assert!(!check.registered);
    assert!(!check.allowed);
}

#[test]
fn spiffe_id_shape() {
    let record = AgentRecord {
        agent_id: "ci-7".to_string(),
        role: "coder".to_string(),
        owner: "ops@org".to_string(),
        status: AgentStatus::Active,
        created_at: String::new(),
        public_key: String::new(),
        attestation: None,
        last_heartbeat: None,
        suspended_reason: None,
        decommissioned_at: None,
        pid: None,
        last_seen: None,
    };
    assert_eq!(
        spiffe_id(&record, "org.example"),
        "spiffe://org.example/agent/coder/ci-7"
    );
}

#[test]
fn heartbeat_updates_liveness_and_reports_no_false_drift() {
    let _iso = isolated();
    register("hb-1", "coder", "yves@org").expect("register");
    let drift = heartbeat("hb-1").expect("heartbeat");
    assert!(
        drift.is_none(),
        "same binary+config must not drift: {drift:?}"
    );
    assert!(get("hb-1").expect("rec").last_heartbeat.is_some());
    assert!(heartbeat("ghost").is_err());
}

#[test]
fn gc_decommissions_dead_pid() {
    let _iso = isolated();
    register("dead-pid", "coder", "yves@org").expect("register");
    super::with_registry(|reg| {
        let record = reg.get_mut("dead-pid").expect("registered");
        record.pid = Some(99_999_999);
        Ok(())
    })
    .expect("set dead pid");
    assert_eq!(gc().expect("gc"), 1);
    assert_eq!(
        get("dead-pid").expect("record").status,
        AgentStatus::Decommissioned
    );
}

#[test]
fn gc_leaves_active_alive() {
    let _iso = isolated();
    register("alive-pid", "coder", "yves@org").expect("register");
    assert_eq!(gc().expect("gc"), 0);
    assert_eq!(
        get("alive-pid").expect("record").status,
        AgentStatus::Active
    );
}
