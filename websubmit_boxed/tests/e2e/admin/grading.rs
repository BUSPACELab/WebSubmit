//! The admin's two reports: the list of registered users and the grading
//! report built from their answers.
use rocket::http::Status;

use crate::e2e::common::*;

#[test]
fn the_user_list_shows_every_registered_user() {
    let admin = client(ADMIN);
    let body = ok_body(admin.get("/admin/users").dispatch());

    assert!(body.contains("Registered users:"));
    assert!(body.contains(ADMIN));
    for student in STUDENTS {
        assert!(body.contains(student), "{} is missing", student);
    }
    // Everyone seeded consented, and only the admin is an admin. Asserted per
    // row: other tests register users of their own, so a count over the whole
    // page would depend on which of them has run.
    assert_eq!(user_row(&body, ADMIN), ("yes", "yes"));
    for student in STUDENTS {
        assert_eq!(user_row(&body, student), ("no", "yes"), "row for {}", student);
    }
}

/// The (admin?, consent?) cells of one user's row on the user list.
fn user_row<'a>(body: &'a str, email: &str) -> (&'a str, &'a str) {
    let row = body
        .split("<tr>")
        .find(|row| row.contains(email))
        .unwrap_or_else(|| panic!("no row for {}", email));
    let mut cells = row
        .split("<td>")
        .skip(2)
        .map(|cell| if cell.contains("yes") { "yes" } else { "no" });
    (cells.next().unwrap(), cells.next().unwrap())
}

#[test]
fn the_user_list_never_shows_an_api_key() {
    let admin = client(ADMIN);
    let body = ok_body(admin.get("/admin/users").dispatch());
    for email in [ADMIN, CORINN, ARTEM, ALEX, SARAH, ALLEN] {
        assert!(
            !body.contains(&apikey(email)),
            "the API key of {} leaked onto the user list",
            email
        );
    }
}

#[test]
fn the_user_list_is_closed_to_students() {
    let student = client(ALEX);
    let response = student.get("/admin/users").dispatch();
    assert_ne!(response.status(), Status::Ok);
}

#[test]
fn the_grading_report_covers_every_user_and_lecture() {
    let admin = client(ADMIN);
    let body = ok_body(admin.get("/admin/grading").dispatch());

    assert!(body.contains("Grading report"));
    assert!(body.contains(ADMIN));
    for student in STUDENTS {
        assert!(body.contains(student), "{} is missing from the report", student);
    }
    for lecture in 1..=3 {
        assert!(
            body.contains(&format!("Lecture {} (/2)", lecture)),
            "lecture {} is missing its column",
            lecture
        );
    }

    // The admin answers nothing, so their row is all zeros; the CSV repeats it.
    assert!(body.contains(&format!("\"{}\",0", ADMIN)));
}

#[test]
fn the_grading_report_is_closed_to_students() {
    for student in [ALEX, CORINN] {
        let client = client(student);
        let response = client.get("/admin/grading").dispatch();
        assert_ne!(response.status(), Status::Ok, "{} read the report", student);
    }
}
