//! The GDPR "access my data" page: what a logged-in user can see about
//! themselves, and that it never leaks their API key.
use rocket::http::Status;

use crate::e2e::common::*;

#[test]
fn a_student_sees_their_own_data_and_never_their_api_key() {
    let client = client(ARTEM);
    let body = ok_body(client.get("/access").dispatch());

    assert!(body.contains(ARTEM));
    // Seeded in common::seed(): Artem presents lecture 2 and has answered
    // both of its questions. Checked without the apostrophe: the template
    // HTML-escapes it to `&#x27;`.
    assert_eq!(body.matches("seeded answer").count(), 2);
    assert!(body.contains("API KEY hidden for security"));
    assert!(!body.contains(&apikey(ARTEM)));
}

#[test]
fn the_access_page_is_closed_to_anonymous_visitors() {
    let client = anonymous();
    let response = client.get("/access").dispatch();
    assert_ne!(response.status(), Status::Ok);
}
