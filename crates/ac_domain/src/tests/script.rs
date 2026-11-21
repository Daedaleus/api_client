use super::*;

#[test]
fn blank_scripts_count_as_none() {
    let mut scripts = Scripts::default();
    assert!(scripts.is_empty());
    scripts.post_response = "  \n".into();
    assert!(scripts.is_empty());
    assert_eq!(scripts.get(ScriptPhase::PostResponse), None);
    scripts.pre_request = "log(1)".into();
    assert!(!scripts.is_empty());
    assert_eq!(scripts.get(ScriptPhase::PreRequest), Some("log(1)"));
}
