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

    let consent: Option<bool> = mysql::prelude::Queryable::exec_first(
        &mut db(),
        "SELECT consent FROM users WHERE email = ?",
        (email,),
    )
    .unwrap();
    assert_eq!(consent, Some(true));
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
