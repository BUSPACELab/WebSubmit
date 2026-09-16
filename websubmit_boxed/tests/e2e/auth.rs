//! Registration, API key login, and what an anonymous visitor may reach.
use rocket::http::Status;

use super::common::*;

#[test]
fn login_page_renders_the_class_name() {
    let client = anonymous();
    let body = ok_body(client.get("/login").dispatch());
    assert!(body.contains(&format!("Welcome to the {} submission system!", CLASS)));
    // Both forms are on the page: register, and log in with a key.
    assert!(body.contains("action=\"/apikey/generate\""));
    assert!(body.contains("action=\"/apikey/check\""));
}

#[test]
fn index_sends_an_anonymous_visitor_to_login() {
    let client = anonymous();
    let response = client.get("/").dispatch();
    assert_eq!(response.status(), Status::SeeOther);
    assert_eq!(response.headers().get_one("Location"), Some("/login"));
}

#[test]
fn index_sends_a_logged_in_user_to_the_lecture_list() {
    let client = client(ALEX);
    let response = client.get("/").dispatch();
    assert_eq!(response.status(), Status::SeeOther);
    assert_eq!(response.headers().get_one("Location"), Some("/leclist"));
}

#[test]
fn registering_stores_the_user_and_acknowledges_the_email() {
    let email = "newcomer@brown.edu";
    let client = anonymous();
    let body = ok_body(post(
        &client,
        "/apikey/generate",
        format!("email={}&consent=true", urlencode(email)),
    ));

    // The page confirms where the key went, and never shows the key itself.
    assert!(body.contains(email));
    assert!(!body.contains(&apikey(email)));

    let consent: Option<(bool, String)> = mysql::prelude::Queryable::exec_first(
        &mut db(),
        "SELECT consent, email FROM users WHERE email = ?",
        (email,),
    )
    .unwrap();
    assert_eq!(consent.map(|(consent, _)| consent), Some(true));
}

#[test]
fn registering_without_consent_stores_false() {
    // An unchecked consent checkbox submits no `consent` field at all, rather
    // than `consent=false`.
    let email = "declines@brown.edu";
    let client = anonymous();
    let body = ok_body(post(
        &client,
        "/apikey/generate",
        format!("email={}", urlencode(email)),
    ));
    assert!(body.contains(email));

    let consent: Option<(bool, String)> = mysql::prelude::Queryable::exec_first(
        &mut db(),
        "SELECT consent, email FROM users WHERE email = ?",
        (email,),
    )
    .unwrap();
    assert_eq!(consent.map(|(consent, _)| consent), Some(false));
}

#[test]
fn resetting_the_api_key_keeps_identity_but_updates_consent() {
    let email = "resetter@brown.edu";
    let client = anonymous();

    post(
        &client,
        "/apikey/generate",
        format!("email={}&consent=true", urlencode(email)),
    );
    let original_key = apikey(email);

    // Resetting is just registering again: the key is a deterministic hash of
    // the email, so it comes back unchanged, but this submission declines
    // consent (the field is absent, as an unchecked box would send it).
    post(
        &client,
        "/apikey/generate",
        format!("email={}", urlencode(email)),
    );

    let row: Option<(String, String, bool)> = mysql::prelude::Queryable::exec_first(
        &mut db(),
        "SELECT apikey, email, consent FROM users WHERE email = ?",
        (email,),
    )
    .unwrap();
    let (stored_key, stored_email, consent) = row.expect("user should still exist after reset");

    assert_eq!(stored_key, original_key, "resetting must not change the API key");
    assert_eq!(stored_email, email, "resetting must not change the email");
    assert!(!consent, "resetting should update consent to the new choice");
}

#[test]
fn registering_twice_yields_the_same_key() {
    let email = "repeat@brown.edu";
    let client = anonymous();
    let first = register(&client, email);
    let second = register(&client, email);
    assert_eq!(first, second);

    let rows: Vec<String> = mysql::prelude::Queryable::exec(
        &mut db(),
        "SELECT email FROM users WHERE email = ?",
        (email,),
    )
    .unwrap();
    assert_eq!(rows.len(), 1, "re-registering must not duplicate the user");
}

#[test]
fn an_unknown_key_does_not_log_anyone_in() {
    let client = anonymous();
    let response = login(&client, "not-a-real-key");
    assert_eq!(response.status(), Status::SeeOther);
    assert_eq!(response.headers().get_one("Location"), Some("/"));

    // ... and the session is still anonymous.
    let response = client.get("/leclist").dispatch();
    assert_ne!(response.status(), Status::Ok);
}

#[test]
fn pages_behind_the_api_key_reject_anonymous_visitors() {
    let client = anonymous();
    for uri in ["/leclist", "/questions/1", "/answers/presenters/1"] {
        let response = client.get(uri).dispatch();
        assert_ne!(response.status(), Status::Ok, "{} was reachable", uri);
    }
}
