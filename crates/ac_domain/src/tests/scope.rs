use super::*;

#[test]
fn stronger_lists_win_and_unusable_rows_do_not_count() {
    let collection = [
        Variable::plain("host", "collection"),
        Variable::plain("a", "1"),
    ];
    let environment = [
        Variable::plain("host", "environment"),
        Variable {
            enabled: false,
            ..Variable::plain("a", "disabled")
        },
        Variable {
            secret: true,
            ..Variable::plain("token", "")
        },
    ];
    let outer = [Variable::plain("host", "outer"), Variable::plain("b", "2")];
    let inner = [Variable::plain(" host ", "inner")];
    let request = [Variable::plain("c", "3")];

    let values = Scope::new()
        .then(&collection)
        .then(&environment)
        .then(&outer)
        .then(&inner)
        .then(&request)
        .values();
    assert_eq!(values.get("host"), Some(&"inner"));
    assert_eq!(values.get("a"), Some(&"1"));
    assert_eq!(values.get("b"), Some(&"2"));
    assert_eq!(values.get("c"), Some(&"3"));
    assert_eq!(values.get("token"), None);
    assert!(Scope::new().values().is_empty());
}
