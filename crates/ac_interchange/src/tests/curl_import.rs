use super::*;

fn import(text: &str) -> Imported {
    from_curl(text).unwrap()
}

#[test]
fn a_simple_get() {
    let imported = import("curl https://api.example.com/items");
    assert_eq!(imported.request.method, HttpMethod::Get);
    assert_eq!(imported.request.url, "https://api.example.com/items");
    assert_eq!(imported.request.body, Body::None);
    assert_eq!(imported.request.auth, Auth::Inherit);
    assert_eq!(imported.ignored, Vec::<String>::new());
}

#[test]
fn a_command_from_the_browser_with_continued_lines() {
    let text = "curl 'https://api.example.com/login' \\\n  -H 'Content-Type: application/json' \\\n  -H 'Accept: */*' \\\n  --data-raw '{\"user\":\"alice\"}' \\\n  --compressed";
    let imported = import(text);
    assert_eq!(imported.request.method, HttpMethod::Post);
    assert_eq!(imported.request.headers.len(), 2);
    assert_eq!(imported.request.headers[1], entry("Accept", "*/*"));
    assert_eq!(
        imported.request.body,
        Body::Json("{\"user\":\"alice\"}".into())
    );
    assert_eq!(imported.ignored, ["--compressed"]);
}

#[test]
fn windows_continuations_and_attached_values() {
    for text in [
        "curl -XPUT \"https://x.example\" ^\r\n -H\"X-A: 1\"",
        "curl.exe --request=PUT https://x.example `\n --header=\"X-A: 1\"",
    ] {
        let imported = import(text);
        assert_eq!(imported.request.method, HttpMethod::Put, "{text}");
        assert_eq!(imported.request.headers, [entry("X-A", "1")], "{text}");
    }
}

#[test]
fn form_data_json_text_and_files() {
    let form = import("curl https://x.example -d 'a=1&b=tea%20pot' --data-urlencode 'c=x y'");
    assert_eq!(
        form.request.body,
        Body::Form(vec![
            FormField {
                name: "a".into(),
                value: "1".into(),
                enabled: true
            },
            FormField {
                name: "b".into(),
                value: "tea pot".into(),
                enabled: true
            },
            FormField {
                name: "c".into(),
                value: "x y".into(),
                enabled: true
            },
        ])
    );
    assert_eq!(
        import("curl https://x.example -d '[1,2]'").request.body,
        Body::Json("[1,2]".into())
    );
    assert_eq!(
        import("curl https://x.example -H 'Content-Type: text/csv' -d 'a,b'")
            .request
            .body,
        Body::Text("a,b".into())
    );
    assert_eq!(
        import("curl https://x.example --data-binary @payload.bin")
            .request
            .body,
        Body::Binary("payload.bin".into())
    );
    assert_eq!(
        import("curl https://x.example --data-raw @literal")
            .request
            .body,
        Body::Form(vec![FormField {
            name: "@literal".into(),
            value: String::new(),
            enabled: true
        }])
    );
    let multipart = import(
        "curl https://x.example -F title=Tea -F 'avatar=@a.png;type=image/png' --form-string 'note=@not-a-file'",
    );
    assert_eq!(
        multipart.request.body,
        Body::Multipart(vec![
            MultipartField {
                name: "title".into(),
                value: "Tea".into(),
                enabled: true,
                file: false
            },
            MultipartField {
                name: "avatar".into(),
                value: "a.png".into(),
                enabled: true,
                file: true
            },
            MultipartField {
                name: "note".into(),
                value: "@not-a-file".into(),
                enabled: true,
                file: false
            },
        ])
    );
}

#[test]
fn auth_cookies_and_the_certificate_check() {
    let imported = import(
        "curl -u alice:s3cr3t -A ua/1 -b 'a=1' -e https://ref.example -k --url https://x.example -o out.txt -L extra",
    );
    assert_eq!(
        imported.request.auth,
        Auth::Basic {
            username: "alice".into(),
            password: "s3cr3t".into()
        }
    );
    assert_eq!(
        imported.request.headers,
        [
            entry("User-Agent", "ua/1"),
            entry("Cookie", "a=1"),
            entry("Referer", "https://ref.example")
        ]
    );
    assert_eq!(imported.request.certificate_check, CertificateCheck::Off);
    assert_eq!(imported.request.url, "https://x.example");
    assert_eq!(imported.ignored, ["-o out.txt", "-L", "extra"]);
    assert_eq!(
        import("curl -u bob https://x.example").request.auth,
        Auth::Basic {
            username: "bob".into(),
            password: String::new()
        }
    );
}

#[test]
fn mistakes_are_reported() {
    assert_eq!(from_curl("wget https://x.example"), Err(CurlError::NotCurl));
    assert_eq!(from_curl(""), Err(CurlError::NotCurl));
    assert_eq!(from_curl("curl 'https://x"), Err(CurlError::Quotes));
    assert_eq!(from_curl("curl -H 'A: 1'"), Err(CurlError::NoUrl));
    assert_eq!(
        from_curl("curl https://x.example -H"),
        Err(CurlError::MissingValue("-H".into()))
    );
    assert_eq!(
        from_curl("curl -X 'BAD METHOD' https://x.example"),
        Err(CurlError::Method("BAD METHOD".into()))
    );
    // A header line without a colon is dropped.
    let headers = import("curl https://x.example -H nonsense").request.headers;
    assert_eq!(headers.len(), 0);
}
