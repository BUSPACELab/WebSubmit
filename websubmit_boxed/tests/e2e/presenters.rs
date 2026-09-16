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
    // Artem presents lecture 2 only. This test owns a scratch lecture of its
    // own (rather than reusing 1/2/3) so its answer doesn't collide with
    // what other tests assume about those. Sarah's answer gives it real
    // content: an empty lecture has no policy-protected data for Sesame to
    // refuse Artem, so the check would trivially (and wrongly) pass.
    let admin = client(ADMIN);
    post(&admin, "/admin/lec/add", "lec_id=97&lec_label=Someone+elses+lecture");
    post(&admin, "/admin/lec/97", "q_prompt=Scratch+question");

    let sarah = client(SARAH);
    post(
        &sarah,
        "/questions/97",
        answers_body(&question_ids(97), "Sarah's answer for lecture 97"),
    );

    let artem = client(ARTEM);
    let response = artem.get("/answers/presenters/97").dispatch();
    assert_ne!(response.status(), Status::Ok);
}

#[test]
fn a_student_who_presents_nothing_is_refused() {
    // Two scratch lectures of this test's own: 98 has a presenter (Corinn,
    // not Allen), 99 has none at all. Sarah answers both so there is real
    // content behind the refusal (see a_presenter_of_one_lecture_may_not_
    // read_another for why an empty lecture wouldn't test anything).
    let admin = client(ADMIN);
    post(&admin, "/admin/lec/add", "lec_id=98&lec_label=Someone+elses+lecture");
    post(&admin, "/admin/lec/98", "q_prompt=Scratch+question");
    post(
        &admin,
        "/admin/lec/edit/98",
        format!("lec_name=Someone+elses+lecture&lec_presenters={}", urlencode(CORINN)),
    );
    post(&admin, "/admin/lec/add", "lec_id=99&lec_label=Unpresented+lecture");
    post(&admin, "/admin/lec/99", "q_prompt=Scratch+question");

    let sarah = client(SARAH);
    post(
        &sarah,
        "/questions/98",
        answers_body(&question_ids(98), "Sarah's answer for lecture 98"),
    );
    post(
        &sarah,
        "/questions/99",
        answers_body(&question_ids(99), "Sarah's answer for lecture 99"),
    );

    let allen = client(ALLEN);
    for lecture in [98, 99] {
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
