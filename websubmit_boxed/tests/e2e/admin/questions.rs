//! Adding and editing a lecture's questions.
use rocket::http::Status;

use crate::e2e::common::*;

#[test]
fn the_admin_can_add_questions_to_a_lecture() {
    let admin = client(ADMIN);
    post(&admin, "/admin/lec/add", "lec_id=94&lec_label=Question+tests");

    for question in 1..=2 {
        let response = post(
            &admin,
            "/admin/lec/94",
            format!("q_prompt=Prompt+number+{}", question),
        );
        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(response.headers().get_one("Location"), Some("/admin/lec/94"));
    }

    // They are numbered within the lecture, in the order they were added.
    let body = ok_body(admin.get("/admin/lec/94").dispatch());
    assert!(body.contains("1: Prompt number 1"));
    assert!(body.contains("2: Prompt number 2"));

    // ... and students are offered a box for each.
    let student = client(ALEX);
    let body = ok_body(student.get("/questions/94").dispatch());
    assert_eq!(question_ids(94).len(), 2);
    for id in question_ids(94) {
        assert!(body.contains(&format!("name=\"answers.{}\"", id)));
    }
}

#[test]
fn the_admin_can_edit_a_question() {
    let admin = client(ADMIN);
    post(&admin, "/admin/lec/add", "lec_id=95&lec_label=Edit+tests");
    post(&admin, "/admin/lec/95", "q_prompt=The+original+prompt");
    let id = question_ids(95)[0];

    let body = ok_body(admin.get(format!("/admin/lec/95/{}", id)).dispatch());
    assert!(body.contains("Edit question 1"));
    assert!(body.contains("The original prompt"));
    assert!(body.contains(&format!("action=\"/admin/lec/editq/95/{}\"", id)));

    let response = post(
        &admin,
        format!("/admin/lec/editq/95/{}", id),
        "q_prompt=The+replacement+prompt",
    );
    assert_eq!(response.status(), Status::SeeOther);

    let body = ok_body(admin.get("/admin/lec/95").dispatch());
    assert!(body.contains("The replacement prompt"));
    assert!(!body.contains("The original prompt"));
}

#[test]
fn questions_are_not_editable_by_students() {
    let admin = client(ADMIN);
    post(&admin, "/admin/lec/add", "lec_id=96&lec_label=Guarded");
    post(&admin, "/admin/lec/96", "q_prompt=Guarded+prompt");
    let id = question_ids(96)[0];

    let student = client(ALEX);
    assert_ne!(
        student.get(format!("/admin/lec/96/{}", id)).dispatch().status(),
        Status::Ok
    );
    assert_ne!(
        post(&student, "/admin/lec/96", "q_prompt=Added+by+a+student").status(),
        Status::Ok
    );
    assert_ne!(
        post(
            &student,
            format!("/admin/lec/editq/96/{}", id),
            "q_prompt=Edited+by+a+student"
        )
        .status(),
        Status::Ok
    );

    let prompts: Vec<String> = mysql::prelude::Queryable::exec(
        &mut db(),
        "SELECT question FROM questions WHERE lecture_id = ?",
        (96u64,),
    )
    .unwrap();
    assert_eq!(prompts, vec![String::from("Guarded prompt")]);
}
