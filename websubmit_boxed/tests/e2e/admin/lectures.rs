//! Adding a lecture, and editing its title and presenters.
use rocket::http::Status;

use crate::e2e::common::*;

#[test]
fn the_add_lecture_form_is_admin_only() {
    let admin = client(ADMIN);
    let body = ok_body(admin.get("/admin/lec/add").dispatch());
    assert!(body.contains("Admin: add lecture"));
    assert!(body.contains("name=\"lec_id\""));
    assert!(body.contains("name=\"lec_label\""));

    let student = client(ALEX);
    let response = student.get("/admin/lec/add").dispatch();
    assert_ne!(response.status(), Status::Ok);

    let response = post(&student, "/admin/lec/add", "lec_id=90&lec_label=Sneaky");
    assert_ne!(response.status(), Status::Ok);
    assert!(lecture_label(90).is_none(), "a student added a lecture");
}

#[test]
fn the_admin_can_add_a_lecture() {
    let admin = client(ADMIN);
    let response = post(&admin, "/admin/lec/add", "lec_id=91&lec_label=Added+lecture");
    assert_eq!(response.status(), Status::SeeOther);
    assert_eq!(response.headers().get_one("Location"), Some("/leclist"));

    assert_eq!(lecture_label(91), Some(String::from("Added lecture")));

    // A lecture with no questions still shows up, with a zero denominator.
    let body = ok_body(admin.get("/leclist").dispatch());
    assert!(body.contains("<a href=\"/questions/91\">Added lecture</a> (0/0)"));
}

#[test]
fn the_lecture_admin_page_shows_its_questions_and_presenters() {
    let admin = client(ADMIN);
    let body = ok_body(admin.get("/admin/lec/1").dispatch());

    assert!(body.contains("Lecture 1 admin"));
    assert!(body.contains("value=\"Lecture 1\""));
    assert!(body.contains(&format!("name=\"lec_presenters\" value=\"{}\"", CORINN)));
    assert!(body.contains("1: Lecture 1 question 1"));
    assert!(body.contains("2: Lecture 1 question 2"));
    for id in question_ids(1) {
        assert!(body.contains(&format!("href=\"/admin/lec/1/{}\"", id)));
    }
}

#[test]
fn the_lecture_admin_page_is_closed_to_students() {
    let student = client(SARAH);
    let response = student.get("/admin/lec/1").dispatch();
    assert_ne!(response.status(), Status::Ok);
}

#[test]
fn editing_a_lecture_sets_its_title_and_replaces_its_presenters() {
    let admin = client(ADMIN);
    post(&admin, "/admin/lec/add", "lec_id=92&lec_label=Before+edit");

    let response = post(
        &admin,
        "/admin/lec/edit/92",
        format!(
            "lec_name=After+edit&lec_presenters={},{}",
            urlencode(SARAH),
            urlencode(ALLEN)
        ),
    );
    assert_eq!(response.status(), Status::SeeOther);
    assert_eq!(response.headers().get_one("Location"), Some("/admin/lec/92"));

    assert_eq!(lecture_label(92), Some(String::from("After edit")));
    assert_eq!(presenters(92), vec![String::from(SARAH), String::from(ALLEN)]);

    // Sarah can now read lecture 92's answers as its presenter.
    let sarah = client(SARAH);
    assert_eq!(
        sarah.get("/answers/presenters/92").dispatch().status(),
        Status::Ok
    );

    // Editing again replaces the presenter list wholesale.
    post(
        &admin,
        "/admin/lec/edit/92",
        format!("lec_name=After+edit&lec_presenters={}", urlencode(ALLEN)),
    );
    assert_eq!(presenters(92), vec![String::from(ALLEN)]);
    assert_ne!(
        sarah.get("/answers/presenters/92").dispatch().status(),
        Status::Ok
    );
}

#[test]
fn editing_a_lecture_is_closed_to_students() {
    let admin = client(ADMIN);
    post(&admin, "/admin/lec/add", "lec_id=93&lec_label=Untouched");

    let student = client(ARTEM);
    let response = post(
        &student,
        "/admin/lec/edit/93",
        format!("lec_name=Hijacked&lec_presenters={}", urlencode(ARTEM)),
    );
    assert_ne!(response.status(), Status::Ok);
    assert_eq!(lecture_label(93), Some(String::from("Untouched")));
    assert!(presenters(93).is_empty());
}

fn lecture_label(id: u64) -> Option<String> {
    let row: Option<(String, u64)> = mysql::prelude::Queryable::exec_first(
        &mut db(),
        "SELECT label, id FROM lectures WHERE id = ?",
        (id,),
    )
    .unwrap();
    row.map(|(label, _)| label)
}

fn presenters(lecture: u64) -> Vec<String> {
    let data: Vec<(i64, i64, String)> = mysql::prelude::Queryable::exec(
        &mut db(),
        "SELECT * FROM presenters WHERE lecture_id = ? ORDER BY id",
        (lecture,),
    )
    .unwrap();

    data.into_iter()
        .map(|(_, _, email)| email)
        .collect()
}
