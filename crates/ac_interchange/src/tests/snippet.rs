use std::path::PathBuf;
use std::time::Duration;

use ac_domain::exchange::{Header, Part, PartContent, RequestBody};
use ac_domain::protocol::HttpMethod;
use ac_domain::transport::EffectiveTransport;
use url::Url;

use super::*;

const ALL: [SnippetLanguage; 6] = [
    SnippetLanguage::Curl,
    SnippetLanguage::JavaScriptFetch,
    SnippetLanguage::JavaScriptAxios,
    SnippetLanguage::PythonRequests,
    SnippetLanguage::GoNetHttp,
    SnippetLanguage::Httpie,
];

fn request(body: Option<RequestBody>) -> ResolvedRequest {
    ResolvedRequest {
        transport: EffectiveTransport::default(),
        method: HttpMethod::Post,
        url: Url::parse("https://api.example.com/items?q=tea").unwrap(),
        headers: vec![
            Header::new("Authorization", "Bearer it's \"quoted\""),
            Header::new("Content-Type", "application/json"),
        ],
        body,
        verify_tls: true,
    }
}

fn snippet(request: &ResolvedRequest, language: SnippetLanguage) -> String {
    to_snippet(request, language)
}

#[test]
fn every_language_has_method_url_headers_and_body() {
    let json = request(Some(RequestBody::Bytes(br#"{"name":"tea"}"#.to_vec())));
    for language in ALL {
        let code = snippet(&json, language);
        // cURL implies POST from the body.
        assert!(
            code.contains("POST") || language == SnippetLanguage::Curl,
            "{language:?}: {code}"
        );
        assert!(
            code.contains("https://api.example.com/items?q=tea"),
            "{language:?}: {code}"
        );
        assert!(code.contains("Authorization"), "{language:?}: {code}");
        assert!(code.contains("name"), "{language:?}: {code}");
    }
    // Quotes and special characters stay valid string literals.
    let fetch = snippet(&json, SnippetLanguage::JavaScriptFetch);
    assert!(
        fetch.contains(r#""Authorization": "Bearer it's \"quoted\"""#),
        "{fetch}"
    );
    assert!(fetch.contains(r#"body: "{\"name\":\"tea\"}""#), "{fetch}");
    let python = snippet(&json, SnippetLanguage::PythonRequests);
    assert!(
        python.contains(r#"data="{\"name\":\"tea\"}".encode("utf-8")"#),
        "{python}"
    );
    assert!(python.contains("timeout=30"), "{python}");
    let go = snippet(&json, SnippetLanguage::GoNetHttp);
    assert!(go.contains("strings.NewReader("), "{go}");
    assert!(go.contains("\"time\""), "{go}");
    assert!(go.contains("Timeout: 30 * time.Second"), "{go}");
    let httpie = snippet(&json, SnippetLanguage::Httpie);
    assert!(httpie.starts_with("http --follow --timeout=30"), "{httpie}");
    assert!(
        httpie.contains(r#"'Authorization:Bearer it'\''s "quoted"'"#),
        "{httpie}"
    );
    assert_eq!(
        snippet(&json, SnippetLanguage::Curl),
        crate::curl::to_curl(&json)
    );
}

#[test]
fn files_and_forms_are_referenced_by_path() {
    let file = request(Some(RequestBody::File(PathBuf::from("data/payload.bin"))));
    assert!(snippet(&file, SnippetLanguage::JavaScriptFetch).contains("import fs from 'node:fs';"));
    assert!(
        snippet(&file, SnippetLanguage::JavaScriptFetch)
            .contains("fs.readFileSync(\"data/payload.bin\")")
    );
    assert!(snippet(&file, SnippetLanguage::JavaScriptAxios).contains("data: fs.readFileSync("));
    assert!(
        snippet(&file, SnippetLanguage::PythonRequests)
            .contains("data=open(\"data/payload.bin\", \"rb\")")
    );
    assert!(snippet(&file, SnippetLanguage::GoNetHttp).contains("os.Open(\"data/payload.bin\")"));
    assert!(snippet(&file, SnippetLanguage::Httpie).contains("< 'data/payload.bin'"));

    let form = request(Some(RequestBody::Multipart(vec![
        Part {
            name: "title".into(),
            content: PartContent::Text("Tea".into()),
        },
        Part {
            name: "image".into(),
            content: PartContent::File(PathBuf::from("img/tea.png")),
        },
    ])));
    let fetch = snippet(&form, SnippetLanguage::JavaScriptFetch);
    assert!(
        fetch.contains("form.append(\"title\", \"Tea\");"),
        "{fetch}"
    );
    assert!(
        fetch.contains("new Blob([fs.readFileSync(\"img/tea.png\")]), \"tea.png\""),
        "{fetch}"
    );
    let python = snippet(&form, SnippetLanguage::PythonRequests);
    assert!(
        python.contains("data={\n        \"title\": \"Tea\","),
        "{python}"
    );
    assert!(
        python.contains("files={\n        \"image\": open(\"img/tea.png\", \"rb\"),"),
        "{python}"
    );
    let go = snippet(&form, SnippetLanguage::GoNetHttp);
    assert!(go.contains("form.WriteField(\"title\", \"Tea\")"), "{go}");
    assert!(
        go.contains("form.CreateFormFile(\"image\", \"tea.png\")"),
        "{go}"
    );
    assert!(go.contains("form.FormDataContentType()"), "{go}");
    let httpie = snippet(&form, SnippetLanguage::Httpie);
    assert!(httpie.contains("--multipart"), "{httpie}");
    assert!(
        httpie.contains("'title=Tea'") && httpie.contains("'image@img/tea.png'"),
        "{httpie}"
    );
}

#[test]
fn redirects_certificate_check_and_time_limit_carry_over() {
    let mut strict = request(None);
    strict.verify_tls = false;
    strict.transport = EffectiveTransport {
        follow_redirects: false,
        timeout: Duration::from_secs(5),
        cookies: true,
    };
    let fetch = snippet(&strict, SnippetLanguage::JavaScriptFetch);
    assert!(
        fetch.contains("redirect: 'manual'") && fetch.contains("NODE_TLS_REJECT_UNAUTHORIZED"),
        "{fetch}"
    );
    assert!(!fetch.contains("body:"), "{fetch}");
    let axios = snippet(&strict, SnippetLanguage::JavaScriptAxios);
    assert!(
        axios.contains("maxRedirects: 0") && axios.contains("rejectUnauthorized: false"),
        "{axios}"
    );
    let python = snippet(&strict, SnippetLanguage::PythonRequests);
    assert!(
        python.contains("allow_redirects=False")
            && python.contains("verify=False")
            && python.contains("timeout=5"),
        "{python}"
    );
    let go = snippet(&strict, SnippetLanguage::GoNetHttp);
    assert!(
        go.contains("ErrUseLastResponse")
            && go.contains("InsecureSkipVerify: true")
            && go.contains(
                "http.NewRequest(\"POST\", \"https://api.example.com/items?q=tea\", nil)"
            ),
        "{go}"
    );
    let httpie = snippet(&strict, SnippetLanguage::Httpie);
    assert!(
        httpie.starts_with("http --verify=no --timeout=5"),
        "{httpie}"
    );
}
