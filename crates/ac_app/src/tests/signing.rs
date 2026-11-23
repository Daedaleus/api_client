use std::path::PathBuf;

use ac_domain::exchange::{Header, RequestBody, ResolvedRequest};
use ac_domain::protocol::HttpMethod;
use md5::Digest as _;
use url::Url;

use super::aws::{AwsCredentials, amz_date, sign};
use super::digest::{Algorithm, Answer, authorization, best_challenge, new_cnonce};
use super::*;

/// 2015-08-30 12:36:00 UTC, the time of AWS's signature test suite.
const AWS_SUITE_TIME: i64 = 1_440_938_160;

fn aws_suite() -> AwsCredentials {
    AwsCredentials {
        access_key_id: "AKIDEXAMPLE".into(),
        secret_access_key: "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY".into(),
        session_token: String::new(),
        region: "us-east-1".into(),
        service: "service".into(),
    }
}

fn request(method: HttpMethod, url: &str) -> ResolvedRequest {
    ResolvedRequest {
        transport: ac_domain::transport::EffectiveTransport::default(),
        method,
        url: Url::parse(url).unwrap(),
        headers: Vec::new(),
        body: None,
        verify_tls: true,
    }
}

fn header<'a>(request: &'a ResolvedRequest, name: &str) -> Option<&'a str> {
    request
        .headers
        .iter()
        .find(|h| h.name.eq_ignore_ascii_case(name))
        .map(|h| h.value.as_str())
}

#[test]
fn aws_signatures_match_the_official_test_suite() {
    // get-vanilla
    let mut vanilla = request(HttpMethod::Get, "https://example.amazonaws.com/");
    sign(&mut vanilla, &aws_suite(), AWS_SUITE_TIME);
    assert_eq!(header(&vanilla, "x-amz-date"), Some("20150830T123600Z"));
    assert_eq!(
        header(&vanilla, "authorization"),
        Some(
            "AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/20150830/us-east-1/service/aws4_request, \
             SignedHeaders=host;x-amz-date, \
             Signature=5fa00fa31553b73ebf1942676e86291e8372ff2a2260956d9b8aae1d763fbf31"
        )
    );
    // get-vanilla-query-order-key-case: parameters sorted.
    let mut ordered = request(
        HttpMethod::Get,
        "https://example.amazonaws.com/?Param2=value2&Param1=value1",
    );
    sign(&mut ordered, &aws_suite(), AWS_SUITE_TIME);
    assert!(
        header(&ordered, "authorization").unwrap().ends_with(
            "Signature=b97d918cfa904a5beff61c982a1b6f458b799221646efd99d3219ec94cdf2500"
        ),
        "{ordered:?}"
    );
}

#[test]
fn aws_signing_adds_token_payload_hash_and_replaces_authorization() {
    let credentials = AwsCredentials {
        session_token: "t0ken".into(),
        service: "s3".into(),
        ..aws_suite()
    };
    let mut put = request(
        HttpMethod::Put,
        "https://bucket.s3.amazonaws.com:8443/a%20b/c?x=1",
    );
    put.headers.push(Header::new("Authorization", "Bearer old"));
    put.headers.push(Header::new("X-Amz-Date", "stale"));
    put.body = Some(RequestBody::Bytes(b"hello".to_vec()));
    sign(&mut put, &credentials, AWS_SUITE_TIME);
    let authorization = header(&put, "authorization").unwrap();
    assert!(
        authorization.starts_with(
            "AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/20150830/us-east-1/s3/aws4_request"
        )
    );
    assert!(
        authorization
            .contains("SignedHeaders=host;x-amz-content-sha256;x-amz-date;x-amz-security-token"),
        "{authorization}"
    );
    assert_eq!(
        put.headers
            .iter()
            .filter(|h| h.name.eq_ignore_ascii_case("authorization")
                || h.name.eq_ignore_ascii_case("x-amz-date"))
            .count(),
        2
    );
    assert_eq!(header(&put, "x-amz-security-token"), Some("t0ken"));
    assert_eq!(
        header(&put, "x-amz-content-sha256"),
        Some("2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824")
    );

    // Streamed bodies are not signed; other services get the header then too.
    let mut upload = request(HttpMethod::Post, "https://api.example.com/upload");
    upload.body = Some(RequestBody::File(PathBuf::from("data.bin")));
    sign(&mut upload, &aws_suite(), AWS_SUITE_TIME);
    assert_eq!(
        header(&upload, "x-amz-content-sha256"),
        Some("UNSIGNED-PAYLOAD")
    );
    // The same request signs the same; another time differently.
    let mut again = request(HttpMethod::Get, "https://example.amazonaws.com/");
    sign(&mut again, &aws_suite(), AWS_SUITE_TIME + 1);
    assert_eq!(header(&again, "x-amz-date"), Some("20150830T123601Z"));
}

#[test]
fn amz_dates_are_utc() {
    assert_eq!(amz_date(0), ("19700101".into(), "19700101T000000Z".into()));
    assert_eq!(
        amz_date(951_825_599),
        ("20000229".into(), "20000229T115959Z".into())
    );
    assert_eq!(
        amz_date(4_102_444_800),
        ("21000101".into(), "21000101T000000Z".into())
    );
}

const RFC_7616: &str = r#"Digest realm="http-auth@example.org", qop="auth, auth-int", algorithm=SHA-256, nonce="7ypf/xlj9XXwfDPEoM4URrv/xwf94BcCAzFZH4GiTo0v", opaque="FQhe/qaU925kfnzjCev0ciny7QMkPqMAFRtzCUYo5tdS""#;
const RFC_7616_MD5: &str = r#"Digest realm="http-auth@example.org", qop="auth, auth-int", algorithm=MD5, nonce="7ypf/xlj9XXwfDPEoM4URrv/xwf94BcCAzFZH4GiTo0v", opaque="FQhe/qaU925kfnzjCev0ciny7QMkPqMAFRtzCUYo5tdS""#;

fn mufasa<'a>(password: &'a str, cnonce: &'a str) -> Answer<'a> {
    Answer {
        username: "Mufasa",
        password,
        method: "GET",
        uri: "/dir/index.html",
        cnonce,
        nonce_count: 1,
    }
}

#[test]
fn digest_answers_match_rfc_7616() {
    let cnonce = "f2/wE4q74E6zIJEtWaHKaf5wv/H5QzzpXusqGemxURZJ";
    // The server offers both; SHA-256 is preferred.
    let challenge = best_challenge([RFC_7616_MD5, RFC_7616]).unwrap();
    assert_eq!(challenge.algorithm, Algorithm::Sha256);
    let answer = authorization(&challenge, &mufasa("Circle of Life", cnonce));
    assert_eq!(
        answer,
        "Digest username=\"Mufasa\", realm=\"http-auth@example.org\", uri=\"/dir/index.html\", \
         algorithm=SHA-256, nonce=\"7ypf/xlj9XXwfDPEoM4URrv/xwf94BcCAzFZH4GiTo0v\", nc=00000001, \
         cnonce=\"f2/wE4q74E6zIJEtWaHKaf5wv/H5QzzpXusqGemxURZJ\", qop=auth, \
         response=\"753927fa0e85d155564e2e272a28d1802ca10daf4496794697cf8db5856cb6c1\", \
         opaque=\"FQhe/qaU925kfnzjCev0ciny7QMkPqMAFRtzCUYo5tdS\""
    );
    let md5 = best_challenge([RFC_7616_MD5]).unwrap();
    assert!(
        authorization(&md5, &mufasa("Circle of Life", cnonce))
            .contains("response=\"8ca523f5e9506fed4657c9700eebdbec\"")
    );
}

#[test]
fn digest_answers_match_rfc_2617_and_the_old_form() {
    // RFC 2617, section 3.5: MD5 without an algorithm parameter.
    let challenge = best_challenge([
        r#"Digest realm="testrealm@host.com", qop="auth,auth-int", nonce="dcd98b7102dd2f0e8b11d0f600bfb0c093", opaque="5ccc069c403ebaf9f0171e9517f40e41""#,
    ])
    .unwrap();
    let answer = authorization(&challenge, &mufasa("Circle Of Life", "0a4f113b"));
    assert!(
        answer.contains("response=\"6629fae49393a05397450978507c4ef1\""),
        "{answer}"
    );
    assert!(!answer.contains("algorithm="));

    // Without qop (RFC 2069): no nc/cnonce.
    let old = best_challenge([r#"Digest realm="r", nonce="n""#]).unwrap();
    assert!(!old.qop_auth);
    let answer = authorization(&old, &mufasa("p", "c"));
    assert!(
        !answer.contains("cnonce") && !answer.contains("opaque"),
        "{answer}"
    );
    let ha1 = hex(&md5::Md5::digest(b"Mufasa:r:p"));
    let ha2 = hex(&md5::Md5::digest(b"GET:/dir/index.html"));
    let expected = hex(&md5::Md5::digest(format!("{ha1}:n:{ha2}").as_bytes()));
    assert!(answer.contains(&expected), "{answer}");
}

#[test]
fn digest_sessions_quotes_and_unsupported_challenges() {
    let session = best_challenge([
        r#"Basic realm="x", Digest realm="a \"q\" b", nonce="n,1", qop=auth, algorithm=MD5-sess"#,
    ])
    .unwrap();
    assert!(session.session);
    assert_eq!(session.realm, "a \"q\" b");
    assert_eq!(session.nonce, "n,1");
    let answer = authorization(
        &session,
        &Answer {
            username: "u\"x",
            ..mufasa("p", "c")
        },
    );
    assert!(answer.contains("username=\"u\\\"x\""), "{answer}");
    assert!(answer.contains("realm=\"a \\\"q\\\" b\""), "{answer}");
    assert!(answer.contains("algorithm=MD5-sess"));
    let ha1 = hex(&md5::Md5::digest(b"u\"x:a \"q\" b:p"));
    let ha1 = hex(&md5::Md5::digest(format!("{ha1}:n,1:c").as_bytes()));
    let ha2 = hex(&md5::Md5::digest(b"GET:/dir/index.html"));
    let expected = hex(&md5::Md5::digest(
        format!("{ha1}:n,1:00000001:c:auth:{ha2}").as_bytes(),
    ));
    assert!(answer.contains(&expected), "{answer}");

    for unsupported in [
        r#"Basic realm="x""#,
        r#"Digest realm="r", nonce="n", qop="auth-int""#,
        r#"Digest realm="r", nonce="n", algorithm=SHA-512-256"#,
        r#"Digest nonce="n""#,
        r#"Digest realm="r""#,
        "Digest",
        "",
    ] {
        assert_eq!(best_challenge([unsupported]), None, "{unsupported}");
    }
    let cnonce = new_cnonce();
    assert_eq!(cnonce.len(), 32);
    assert_ne!(cnonce, new_cnonce());
}

#[test]
fn signers_keep_secrets_out_of_debug_output() {
    let debug = format!(
        "{:?} {:?}",
        Signer::Digest {
            username: "bob".into(),
            password: "hunter2".into()
        },
        Signer::Aws(aws_suite())
    );
    assert!(
        debug.contains("bob") && debug.contains("AKIDEXAMPLE"),
        "{debug}"
    );
    assert!(
        !debug.contains("hunter2") && !debug.contains("EXAMPLEKEY"),
        "{debug}"
    );
}
