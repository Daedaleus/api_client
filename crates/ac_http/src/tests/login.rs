use std::sync::Mutex;

use super::*;

/// What the fake browser does with an opened URL.
type Visit = Box<dyn Fn(&Url) -> Result<(), String> + Send + Sync>;

/// Plays the browser: does what `visit` says with the opened URL.
struct FakeBrowser {
    visit: Visit,
    opened: Mutex<Vec<Url>>,
}

impl FakeBrowser {
    fn new(visit: impl Fn(&Url) -> Result<(), String> + Send + Sync + 'static) -> Self {
        Self {
            visit: Box::new(visit),
            opened: Mutex::new(Vec::new()),
        }
    }
}

impl BrowserOpener for FakeBrowser {
    fn open(&self, url: &Url) -> Result<(), String> {
        self.opened.lock().unwrap().push(url.clone());
        (self.visit)(url)
    }
}

/// A browser that follows the sign-in straight to the redirect, first
/// asking for a favicon and sending a POST, as noise.
fn approving_browser(redirect_query: &'static str) -> FakeBrowser {
    FakeBrowser::new(move |url| {
        let redirect: Url = url
            .query_pairs()
            .find(|(key, _)| key == "redirect_uri")
            .map(|(_, value)| value.parse().unwrap())
            .unwrap();
        tokio::spawn(async move {
            let client = reqwest::Client::new();
            let favicon = redirect.join("/favicon.ico").unwrap();
            let status = client.get(favicon).send().await.unwrap().status();
            assert_eq!(status, 404);
            let _ = client.post(redirect.clone()).send().await;
            let mut target = redirect.clone();
            target.set_query(Some(redirect_query));
            let page = client
                .get(target)
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap();
            assert!(page.contains("&#1000"), "{page}");
        });
        Ok(())
    })
}

fn sign_in_url(redirect_uri: &Url) -> Url {
    let mut url = Url::parse("https://auth.example.com/authorize").unwrap();
    url.query_pairs_mut()
        .append_pair("redirect_uri", redirect_uri.as_str());
    url
}

#[tokio::test]
async fn receives_the_redirect_on_a_free_port() {
    let login = LoopbackLogin::with_opener(
        approving_browser("code=c0de&state=s%20t"),
        Duration::from_secs(10),
    );
    let redirect = Url::parse("http://127.0.0.1:0/callback").unwrap();
    let pending = login.listen(&redirect).await.unwrap();
    let redirect_uri = pending.redirect_uri().clone();
    assert_ne!(redirect_uri.port(), Some(0));
    assert_eq!(redirect_uri.path(), "/callback");
    let params = pending.complete(sign_in_url(&redirect_uri)).await.unwrap();
    assert_eq!(
        params,
        [
            ("code".to_owned(), "c0de".to_owned()),
            ("state".to_owned(), "s t".to_owned())
        ]
    );
    assert_eq!(login.opener.opened.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn a_refused_sign_in_is_passed_on_too() {
    let login = LoopbackLogin::with_opener(
        approving_browser("error=access_denied&state=x"),
        Duration::from_secs(10),
    );
    let redirect = Url::parse("http://localhost:0/").unwrap();
    let pending = login.listen(&redirect).await.unwrap();
    let url = sign_in_url(pending.redirect_uri());
    let params = pending.complete(url).await.unwrap();
    assert_eq!(params[0], ("error".to_owned(), "access_denied".to_owned()));
}

#[tokio::test]
async fn gives_up_when_nobody_signs_in_or_no_browser_starts() {
    let idle = LoopbackLogin::with_opener(FakeBrowser::new(|_| Ok(())), Duration::from_millis(50));
    let redirect = Url::parse("http://127.0.0.1:0/callback").unwrap();
    let pending = idle.listen(&redirect).await.unwrap();
    let url = sign_in_url(pending.redirect_uri());
    assert_eq!(pending.complete(url).await, Err(LoginError::TimedOut));

    let broken = LoopbackLogin::with_opener(
        FakeBrowser::new(|_| Err("no browser".into())),
        Duration::from_secs(1),
    );
    let pending = broken.listen(&redirect).await.unwrap();
    let url = sign_in_url(pending.redirect_uri());
    assert_eq!(
        pending.complete(url).await,
        Err(LoginError::Browser("no browser".into()))
    );
}

#[tokio::test]
async fn listens_only_on_loopback_addresses_that_are_free() {
    let login = LoopbackLogin::with_opener(FakeBrowser::new(|_| Ok(())), Duration::from_secs(1));
    let outside = Url::parse("http://example.com/callback").unwrap();
    assert!(matches!(
        login.listen(&outside).await,
        Err(LoginError::Listen { .. })
    ));

    let first = login
        .listen(&Url::parse("http://[::1]:0/cb").unwrap())
        .await;
    // Machines without IPv6 cannot listen there; then there is nothing to check.
    if let Ok(first) = first {
        let taken = first.redirect_uri().clone();
        assert!(matches!(
            login.listen(&taken).await,
            Err(LoginError::Listen { .. })
        ));
    }
    assert_eq!(
        loopback_address(&Url::parse("http://localhost/cb").unwrap()).unwrap(),
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 80)
    );
}

#[test]
fn the_default_uses_the_system_browser() {
    let login = LoopbackLogin::default();
    assert_eq!(login.timeout, SIGN_IN_TIMEOUT);
}
