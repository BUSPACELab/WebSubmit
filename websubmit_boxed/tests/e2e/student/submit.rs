//! Answering a lecture's questions, and who can read those answers back.
use mysql::prelude::Queryable;
use rocket::http::Status;

use crate::e2e::common::*;

// Alex owns lecture 1 in these tests; no other test writes answers for it.
const LECTURE: u64 = 1;

#[test]
fn the_questions_page_shows_a_box_per_question() {
    let client = client(CORINN);
    let body = ok_body(client.get("/questions/3").dispatch());

    assert!(body.contains("Lecture 3 questions:"));
    assert!(body.contains("Lecture 3 question 1"));
    assert!(body.contains("Lecture 3 question 2"));
    for id in question_ids(3) {
        assert!(body.contains(&format!("name=\"answers.{}\"", id)));
    }
    // Nothing answered yet, so every box offers the placeholder.
    assert_eq!(body.matches("placeholder=").count(), 2);
}

#[test]
fn submitting_answers_stores_them_and_shows_them_again() {
    let ids = question_ids(LECTURE);
    let answer = "Alex answered this one properly";

    let client = client(ALEX);
    let response = post(
        &client,
        format!("/questions/{}", LECTURE),
        answers_body(&ids, answer),
    );
    assert_eq!(response.status(), Status::SeeOther);
    assert_eq!(response.headers().get_one("Location"), Some("/leclist"));

    // The answers come back on the questions page, in the right boxes.
    let body = ok_body(client.get(format!("/questions/{}", LECTURE)).dispatch());
    assert_eq!(body.matches(answer).count(), ids.len());

    // ... and the lecture list now counts them.
    let body = ok_body(client.get("/leclist").dispatch());
    assert!(body.contains(&format!(
        "<a href=\"/questions/{0}\">Lecture {0}</a> (2/2)",
        LECTURE
    )));

    // Stored under the synthesised "{email}-{question_id}" key.
    let stored: Vec<(String, String, u64, String, u64)> = db()
        .exec(
            "SELECT id, answer, lec, email, question_id FROM answers WHERE email = ? AND lec = ? \
             ORDER BY question_id",
            (ALEX, LECTURE),
        )
        .unwrap();
    assert_eq!(stored.len(), ids.len());
    for (id, question) in stored.iter().map(|(id, ..)| id).zip(ids.iter()) {
        assert_eq!(id, &format!("{}-{}", ALEX, question));
    }
    assert!(stored.iter().all(|(_, stored, lec, ..)| stored == answer && *lec == LECTURE));
}

#[test]
fn resubmitting_replaces_the_previous_answer() {
    // A lecture of its own, so no other test's writes land on these rows.
    let ids = question_ids(2);
    let client = client(ALEX);

    post(
        &client,
        "/questions/2",
        answers_body(&ids[..1], "A first attempt at the answer"),
    );
    post(
        &client,
        "/questions/2",
        answers_body(&ids[..1], "A second and better attempt"),
    );

    let stored: Vec<(String, String)> = db()
        .exec(
            "SELECT answer, id FROM answers WHERE id = ?",
            (format!("{}-{}", ALEX, ids[0]),),
        )
        .unwrap();
    let stored: Vec<String> = stored.into_iter().map(|(answer, _)| answer).collect();
    assert_eq!(stored, vec![String::from("A second and better attempt")]);
}

#[test]
fn a_student_does_not_see_another_students_answers() {
    let ids = question_ids(LECTURE);
    let answer = "Alex answered this one properly";
    let alex = client(ALEX);
    post(
        &alex,
        format!("/questions/{}", LECTURE),
        answers_body(&ids, answer),
    );

    // Sarah gets the same questions, with empty boxes and no sign of Alex.
    let sarah = client(SARAH);
    let body = ok_body(sarah.get(format!("/questions/{}", LECTURE)).dispatch());
    for id in &ids {
        assert!(body.contains(&format!("name=\"answers.{}\"", id)));
    }
    assert!(!body.contains(answer));
    assert!(!body.contains(ALEX));
    assert_eq!(body.matches("placeholder=").count(), ids.len());
}
