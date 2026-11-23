use std::time::Duration;

use ac_domain::exchange::Header;

use crate::command::{Command, CommandAck, Query, QueryResult};
use crate::test_support::Fixture;
use crate::testing::{FakeHttpSender, response};
use crate::text::keys;
use crate::view::{CookieKeyView, CookieView, SendContext};

fn cookies(fixture: &Fixture, collection: Option<&str>) -> Vec<CookieView> {
    match fixture
        .app
        .query(Query::Cookies {
            collection_id: collection.map(Into::into),
        })
        .unwrap()
    {
        QueryResult::Cookies { cookies } => cookies,
        other => panic!("expected cookies, got {other:?}"),
    }
}

fn changed(ack: CommandAck) -> Vec<CookieView> {
    match ack {
        CommandAck::CookiesChanged { cookies } => cookies,
        other => panic!("expected changed cookies, got {other:?}"),
    }
}

fn key(cookie: &CookieView) -> CookieKeyView {
    CookieKeyView {
        name: cookie.name.clone(),
        domain: cookie.domain.clone(),
        path: cookie.path.clone(),
    }
}

/// A fixture whose collection got `session=s3cret` from a login.
async fn logged_in() -> Fixture {
    let mut login = response(200, "text/plain", "");
    login.headers.push(Header::new(
        "Set-Cookie",
        "session=s3cret; Path=/; Secure; HttpOnly; Max-Age=3600000000",
    ));
    let fixture = Fixture::with_http(FakeHttpSender::new().reply(Ok(login)));
    let mut events = fixture.app.subscribe();
    let mut request = crate::view::OutgoingRequest::get("https://x.example/login");
    request.context = Some(SendContext {
        collection_id: fixture.collection.clone(),
        request_id: None,
        environment_id: None,
    });
    fixture
        .app
        .execute(Command::SendRequest { request })
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), events.recv())
        .await
        .expect("run finishes")
        .expect("channel open");
    fixture
}

#[tokio::test]
async fn the_manager_lists_edits_deletes_and_clears_cookies() {
    let fixture = logged_in().await;
    let collection = Some(fixture.collection.as_str());
    let listed = cookies(&fixture, collection);
    assert_eq!(listed.len(), 1);
    let session = &listed[0];
    assert_eq!(
        (session.name.as_str(), session.value.as_str()),
        ("session", "s3cret")
    );
    assert_eq!(
        (session.domain.as_str(), session.path.as_str()),
        ("x.example", "/")
    );
    assert!(session.host_only && session.secure && session.http_only);
    assert!(session.expires.is_some());
    // Requests outside the collection have their own jar.
    assert_eq!(cookies(&fixture, None), []);

    // Editing may rename: the original is replaced.
    let mut edited = session.clone();
    edited.name = " token ".into();
    edited.domain = ".X.Example".into();
    let after = changed(
        fixture
            .app
            .execute(Command::SaveCookie {
                collection_id: collection.map(Into::into),
                original: Some(key(session)),
                cookie: edited,
            })
            .unwrap(),
    );
    assert_eq!(after.len(), 1);
    assert_eq!(
        (after[0].name.as_str(), after[0].domain.as_str()),
        ("token", "x.example")
    );

    // Adding one by hand.
    let mut added = after[0].clone();
    added.name = "lang".into();
    added.value = "de".into();
    added.expires = None;
    let after = changed(
        fixture
            .app
            .execute(Command::SaveCookie {
                collection_id: collection.map(Into::into),
                original: None,
                cookie: added,
            })
            .unwrap(),
    );
    assert_eq!(after.len(), 2);

    let after = changed(
        fixture
            .app
            .execute(Command::DeleteCookie {
                collection_id: collection.map(Into::into),
                cookie: key(&after[0]),
            })
            .unwrap(),
    );
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].name, "lang");
    let after = changed(
        fixture
            .app
            .execute(Command::ClearCookies {
                collection_id: collection.map(Into::into),
            })
            .unwrap(),
    );
    assert_eq!(after, []);
}

#[tokio::test]
async fn unusable_cookies_and_unknown_collections_are_refused() {
    let fixture = logged_in().await;
    let session = cookies(&fixture, Some(&fixture.collection)).remove(0);
    for (change, expected) in [
        (
            (|c: &mut CookieView| c.name = "a b".into()) as fn(&mut CookieView),
            keys::COOKIE_NAME,
        ),
        (|c| c.value = "a;b".into(), keys::COOKIE_VALUE),
        (|c| c.domain = " ".into(), keys::COOKIE_DOMAIN),
        (|c| c.path = "api".into(), keys::COOKIE_PATH),
    ] {
        let mut cookie = session.clone();
        change(&mut cookie);
        let error = fixture
            .app
            .execute(Command::SaveCookie {
                collection_id: None,
                original: None,
                cookie,
            })
            .unwrap_err();
        assert_eq!(error.user_text().key, expected);
    }
    // Nothing changed.
    assert_eq!(cookies(&fixture, Some(&fixture.collection)).len(), 1);

    let error = fixture
        .app
        .query(Query::Cookies {
            collection_id: Some("nope".into()),
        })
        .unwrap_err();
    assert_eq!(error.user_text().key, keys::UNKNOWN_COLLECTION);
}
