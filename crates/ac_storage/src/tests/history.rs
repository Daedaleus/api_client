use std::collections::BTreeMap;

use ac_app::history::{HistoryId, HistoryOutcome, SentRequest};

use super::*;

fn entry(url: &str) -> HistoryEntry {
    HistoryEntry {
        id: HistoryId::new(),
        at_ms: 0,
        collection: None,
        request: SentRequest {
            method: "GET".into(),
            url: url.into(),
            headers: Vec::new(),
            body: None,
        },
        outcome: HistoryOutcome::Failed {
            key: "error.request".into(),
            args: BTreeMap::new(),
        },
    }
}

fn urls(history: &FileHistory) -> Vec<String> {
    history
        .entries()
        .unwrap()
        .into_iter()
        .map(|entry| entry.request.url)
        .collect()
}

#[test]
fn entries_are_appended_listed_newest_first_and_cleared() {
    let temp = tempfile::tempdir().unwrap();
    let history = FileHistory::new(&temp.path().join("data"));
    assert_eq!(history.entries().unwrap(), []);
    history.clear().unwrap();

    history.record(entry("https://a/")).unwrap();
    history.record(entry("https://b/")).unwrap();
    assert_eq!(urls(&history), ["https://b/", "https://a/"]);
    let stamped = history.entries().unwrap()[0].at_ms;
    assert!(stamped > 1_700_000_000_000, "{stamped}");
    let text = std::fs::read_to_string(temp.path().join("data").join(HISTORY_FILE)).unwrap();
    assert_eq!(text.lines().count(), 2);
    assert!(text.starts_with("{\"version\":1,"), "{text}");

    history.clear().unwrap();
    assert_eq!(history.entries().unwrap(), []);
}

#[test]
fn old_entries_are_dropped_and_unreadable_lines_backed_up() {
    let temp = tempfile::tempdir().unwrap();
    let history = FileHistory::new(temp.path());
    let file = temp.path().join(HISTORY_FILE);
    std::fs::write(&file, "not json\n{\"version\":9,\"entry\":{}}\n").unwrap();
    for index in 0..=MAX_ENTRIES + SLACK {
        history
            .record(entry(&format!("https://x/{index}")))
            .unwrap();
    }
    let kept = urls(&history);
    assert_eq!(kept.len(), MAX_ENTRIES);
    assert_eq!(kept[0], format!("https://x/{}", MAX_ENTRIES + SLACK));
    // Trimmed once past the slack; the two unreadable lines counted too.
    let lines = std::fs::read_to_string(&file).unwrap().lines().count();
    assert!(
        (MAX_ENTRIES..MAX_ENTRIES + SLACK).contains(&lines),
        "{lines}"
    );
    let backups: Vec<_> = std::fs::read_dir(temp.path())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|item| item.file_name().to_string_lossy().ends_with(".bak"))
        .collect();
    assert_eq!(backups.len(), 1);
    assert!(
        std::fs::read_to_string(backups[0].path())
            .unwrap()
            .starts_with("not json")
    );
}

#[test]
fn unreadable_files_are_errors() {
    let temp = tempfile::tempdir().unwrap();
    // A folder where the file should be.
    std::fs::create_dir(temp.path().join(HISTORY_FILE)).unwrap();
    let history = FileHistory::new(temp.path());
    assert!(history.entries().is_err());
    assert!(history.record(entry("https://a/")).is_err());
    assert!(history.clear().is_err());
}
