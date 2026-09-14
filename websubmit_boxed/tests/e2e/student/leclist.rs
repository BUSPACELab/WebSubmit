//! The lecture list: what a student sees, and the extra links an admin gets.
use crate::e2e::common::*;

#[test]
fn the_lecture_list_shows_every_lecture_and_its_question_count() {
    // Corinn never answers anything, so her counts stay at zero.
    let client = client(CORINN);
    let body = ok_body(client.get("/leclist").dispatch());

    // Each seeded lecture links to its questions and has two of them, none
    // of which Corinn has answered.
    for lecture in 1..=3 {
        assert!(
            body.contains(&format!(
                "<a href=\"/questions/{0}\">Lecture {0}</a> (0/2)",
                lecture
            )),
            "lecture {} is missing or miscounted on the list",
            lecture
        );
    }
}

#[test]
fn students_do_not_see_the_admin_links() {
    let client = client(ALLEN);
    let body = ok_body(client.get("/leclist").dispatch());
    assert!(!body.contains("/admin/lec/add"));
    assert!(!body.contains("admin/users"));
    assert!(!body.contains("href=\"/answers/1\""));
}

#[test]
fn the_admin_sees_the_admin_links() {
    let client = client(ADMIN);
    let body = ok_body(client.get("/leclist").dispatch());
    assert!(body.contains("/admin/lec/add"));
    assert!(body.contains("admin/users"));
    assert!(body.contains("href=\"/answers/1\""));
    assert!(body.contains("href=\"/admin/lec/1\""));
}
