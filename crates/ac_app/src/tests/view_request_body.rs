use super::*;

#[test]
fn every_body_round_trips_and_matches_the_ui_contract() {
    let bodies = [
        Body::None,
        Body::Json("{}".into()),
        Body::Text("hi".into()),
        Body::Form(vec![FormField {
            name: "q".into(),
            value: "tea pot".into(),
            enabled: true,
        }]),
        Body::Multipart(vec![
            MultipartField {
                name: "title".into(),
                value: "Teapot".into(),
                enabled: true,
                file: false,
            },
            MultipartField {
                name: "avatar".into(),
                value: "files/avatar.png".into(),
                enabled: false,
                file: true,
            },
        ]),
        Body::Binary("data/payload.bin".into()),
    ];
    for body in &bodies {
        assert_eq!(&BodyDraft::from_body(body).to_body(), body);
    }
    let json = serde_json::to_value(BodyDraft::from_body(&bodies[5])).unwrap();
    assert_eq!(
        json,
        serde_json::json!({ "kind": "binary", "file": "data/payload.bin" })
    );
    let draft: BodyDraft = serde_json::from_value(serde_json::json!({
        "kind": "multipart",
        "fields": [{ "name": "a", "value": "b", "enabled": true }]
    }))
    .unwrap();
    assert_eq!(
        draft.to_body(),
        Body::Multipart(vec![MultipartField {
            name: "a".into(),
            value: "b".into(),
            enabled: true,
            file: false,
        }])
    );
}
