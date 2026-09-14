//! Who may read a lecture's answers: its presenters (anonymously) and the
//! admin (with the authors' emails).
use rocket::http::Status;

use super::common::*;

#[test]
fn a_presenter_sees_the_answers_without_the_authors() {
    // Sarah owns lecture 2's answers in these tests; Artem presents it.
    let answer = "Sarah wrote this for the discussion";
    let sarah = client(SARAH);
    post(&sarah, "/questions/2", answers_body(&question_ids(2), answer));

    let artem = client(ARTEM);
    let body = ok_body(artem.get("/answers/presenters/2").dispatch());

    assert!(body.contains("Lecture 2 answers:"));
    assert!(body.contains(answer));
    // The presenter's view is anonymous: no author appears anywhere on it.
    for student in STUDENTS {
        assert!(!body.contains(student), "{} was revealed", student);
    }
    assert!(!body.contains("Email"));
}

#[test]
fn a_presenter_of_one_lecture_may_not_read_another() {
    // Artem presents lecture 2 only.
    let artem = client(ARTEM);
    let response = artem.get("/answers/presenters/1").dispatch();
    assert_ne!(response.status(), Status::Ok);
}

#[test]
fn a_student_who_presents_nothing_is_refused() {
    let allen = client(ALLEN);
    // Lecture 3 has no presenter at all, and lecture 1 has someone else's.
    for lecture in [1, 3] {
        let response = allen
            .get(format!("/answers/presenters/{}", lecture))
            .dispatch();
        assert_ne!(
            response.status(),
            Status::Ok,
            "lecture {} was readable by a non-presenter",
            lecture
        );
    }
}

#[test]
fn the_admin_answers_page_names_the_authors() {
    // Allen owns lecture 3's answers in these tests.
    let answer = "Allen had something to say here";
    let allen = client(ALLEN);
    post(&allen, "/questions/3", answers_body(&question_ids(3), answer));

    let admin = client(ADMIN);
    let body = ok_body(admin.get("/answers/3").dispatch());

    assert!(body.contains("Lecture 3 answers:"));
    assert!(body.contains(answer));
    assert!(body.contains(ALLEN), "the admin view identifies the author");
}

#[test]
fn the_admin_answers_page_is_closed_to_students() {
    let client = client(CORINN);
    // Corinn presents lecture 1, which still does not make her an admin.
    let response = client.get("/answers/1").dispatch();
    assert_ne!(response.status(), Status::Ok);
}
