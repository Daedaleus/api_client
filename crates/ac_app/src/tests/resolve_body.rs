use std::collections::HashMap;

use ac_domain::collection::{FormField, MultipartField};
use ac_domain::environment::interpolate;

use super::*;
use crate::text::keys;

fn resolve(
    body: &Body,
    base: Option<&Path>,
) -> Result<(Option<RequestBody>, Vec<Header>), AppError> {
    let variables = HashMap::from([("q", "tea & pot"), ("dir", "files")]);
    let fill = |text: &str| interpolate(text, |name| variables.get(name).copied());
    let mut headers = Vec::new();
    let body = resolve_body(body, &mut headers, base, &fill)?;
    Ok((body, headers))
}

fn field(name: &str, value: &str, enabled: bool) -> FormField {
    FormField {
        name: name.into(),
        value: value.into(),
        enabled,
    }
}

fn base() -> PathBuf {
    std::env::temp_dir().join("shop")
}

#[test]
fn forms_are_url_encoded_with_variables() {
    let form = Body::Form(vec![
        field("q", "{{q}}", true),
        field("off", "x", false),
        field(" ", "nameless", true),
        field(" page ", "2", true),
    ]);
    let (body, headers) = resolve(&form, None).unwrap();
    assert_eq!(
        body,
        Some(RequestBody::Bytes(b"q=tea+%26+pot&page=2".to_vec()))
    );
    assert_eq!(
        headers,
        [Header::new(header::CONTENT_TYPE, media_type::FORM)]
    );
}

#[test]
fn multipart_names_files_by_absolute_path() {
    let multipart = Body::Multipart(vec![
        MultipartField {
            name: "title".into(),
            value: "{{q}}".into(),
            enabled: true,
            file: false,
        },
        MultipartField {
            name: "avatar".into(),
            value: "{{dir}}/avatar.png".into(),
            enabled: true,
            file: true,
        },
        MultipartField {
            name: "off".into(),
            value: "x".into(),
            enabled: false,
            file: false,
        },
    ]);
    let (body, headers) = resolve(&multipart, Some(&base())).unwrap();
    assert_eq!(
        body,
        Some(RequestBody::Multipart(vec![
            Part {
                name: "title".into(),
                content: PartContent::Text("tea & pot".into()),
            },
            Part {
                name: "avatar".into(),
                content: PartContent::File(base().join("files/avatar.png")),
            },
        ]))
    );
    // The adapter sets the multipart type with its boundary.
    assert_eq!(headers, []);
}

#[test]
fn binary_bodies_are_files_relative_to_the_collection() {
    let (body, headers) = resolve(&Body::Binary(" data.bin ".into()), Some(&base())).unwrap();
    assert_eq!(body, Some(RequestBody::File(base().join("data.bin"))));
    assert_eq!(
        headers,
        [Header::new(header::CONTENT_TYPE, media_type::OCTET_STREAM)]
    );

    let absolute = base().join("abs.bin");
    let (body, _) = resolve(&Body::Binary(absolute.display().to_string()), None).unwrap();
    assert_eq!(body, Some(RequestBody::File(absolute)));

    let error = resolve(&Body::Binary("data.bin".into()), None).unwrap_err();
    assert_eq!(error, AppError::RelativeFile("data.bin".into()));
    assert_eq!(error.user_text().key, keys::FILE_RELATIVE);
    assert!(error.to_string().contains("relative"));
}

#[test]
fn an_own_content_type_wins_and_no_body_adds_nothing() {
    let variables: HashMap<&str, &str> = HashMap::new();
    let fill = |text: &str| interpolate(text, |name| variables.get(name).copied());
    let mut headers = vec![Header::new("Content-Type", "application/vnd.custom")];
    resolve_body(&Body::Text("x".into()), &mut headers, None, &fill).unwrap();
    assert_eq!(headers.len(), 1);
    assert_eq!(resolve(&Body::None, None).unwrap(), (None, Vec::new()));
}
