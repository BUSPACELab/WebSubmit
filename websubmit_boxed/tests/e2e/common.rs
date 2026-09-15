//! Shared fixture: an in-process copy of the app, backed by its own database
//! and seeded once per test binary.
//!
//! There is no rollback between tests: they share one database for the whole
//! run, and every write a test makes is visible to every test that follows it.
//! Two rules keep that from mattering, and new tests need to follow both:
//!
//!   1. Writers own disjoint rows. A test that submits answers owns a
//!      (student, lecture) pair no other test writes, and a test that creates
//!      lectures or questions uses a scratch lecture id of its own (90-96 so
//!      far). A test never relies on another test having written something:
//!      whatever it reads, it writes first.
//!   2. Readers assert anchored facts, not totals over the page. "Lecture 1
//!      links to its questions and shows (0/2)" survives another test adding a
//!      lecture; "the page contains three (0/2)" does not.
//!
//! `cargo test` runs this binary single threaded (see .cargo/config.toml), but
//! that only serialises the tests -- it does not order them, so both rules hold
//! regardless. `cargo test -- -Z unstable-options --shuffle-seed N` reorders
//! them on purpose and is the way to check that they still do.
#![allow(dead_code)]

use std::sync::OnceLock;

use mysql::prelude::Queryable;
use rocket::http::{ContentType, Status};
use rocket::local::blocking::LocalResponse;
use sesame_rocket::testing::SesameClient;
use websubmit_boxed::{make_rocket, Config};

pub const ADMIN: &str = "admin@bu.edu";
pub const CORINN: &str = "corinn@brown.edu";
pub const ARTEM: &str = "artem@brown.edu";
pub const ALEX: &str = "alex@brown.edu";
pub const SARAH: &str = "sarah@brown.edu";
pub const ALLEN: &str = "allen@brown.edu";
pub const STUDENTS: [&str; 5] = [CORINN, ARTEM, ALEX, SARAH, ALLEN];

pub const CLASS: &str = "TEST 101";

/// Tests use a database of their own, which seeding drops and recreates, so
/// they never disturb a development one.
const DB_NAME: &str = "websubmit_test";

fn db_user() -> String {
    std::env::var("WEBSUBMIT_TEST_DB_USER").unwrap_or_else(|_| String::from("root"))
}

fn db_password() -> String {
    std::env::var("WEBSUBMIT_TEST_DB_PASSWORD").unwrap_or_else(|_| String::from("password"))
}

fn config(prime: bool) -> Config {
    // One rocket is launched per client, so keep their banners out of the test
    // output unless the runner asks for them.
    if std::env::var("ROCKET_LOG_LEVEL").is_err() {
        std::env::set_var("ROCKET_LOG_LEVEL", "critical");
    }

    // The app mounts these as file servers, so they have to exist.
    let resource_dir = format!("{}/target/e2e-resources", env!("CARGO_MANIFEST_DIR"));
    std::fs::create_dir_all(format!("{}/css", resource_dir)).unwrap();
    std::fs::create_dir_all(format!("{}/js", resource_dir)).unwrap();

    Config {
        class: String::from(CLASS),
        // Unused: the tests drive the app through a local client, which never
        // binds a listener.
        port: 8000,
        db_name: String::from(DB_NAME),
        db_user: db_user(),
        db_password: db_password(),
        admins: vec![String::from(ADMIN)],
        staff: vec![String::from(ADMIN)],
        template_dir: format!("{}/templates", env!("CARGO_MANIFEST_DIR")),
        resource_dir,
        secret: String::from("test-secret"),
        // Tests must not talk to an SMTP server.
        send_emails: false,
        smtp_server: String::from("localhost"),
        smtp_port: 25,
        smtp_user: String::new(),
        smtp_password: String::new(),
        smtp_from: String::from("no-reply@test.invalid"),
        prime,
    }
}

/// A direct database connection, for asserting on stored state. Seeding first
/// means a test may reach for the database before it touches the app.
pub fn db() -> mysql::Conn {
    seed();
    raw_db()
}

fn raw_db() -> mysql::Conn {
    mysql::Conn::new(
        mysql::Opts::from_url(&format!(
            "mysql://{}:{}@127.0.0.1/{}",
            db_user(),
            db_password(),
            DB_NAME
        ))
        .unwrap(),
    )
    .unwrap()
}

/// The API key of an already registered user, read out of the database: the
/// application itself only ever emails it.
pub fn apikey(email: &str) -> String {
    seed();
    raw_apikey(email)
}

fn raw_apikey(email: &str) -> String {
    raw_db()
        .exec_first::<String, _, _>("SELECT apikey FROM users WHERE email = ?", (email,))
        .unwrap()
        .unwrap_or_else(|| panic!("{} is not registered", email))
}

/// The question ids of a lecture, in the order they were added.
pub fn question_ids(lecture: u64) -> Vec<u64> {
    db().exec(
        "SELECT id FROM questions WHERE lecture_id = ? ORDER BY question_number",
        (lecture,),
    )
    .unwrap()
}

/// An app instance with no session.
pub fn anonymous() -> SesameClient {
    seed();
    SesameClient::tracked(make_rocket(config(false))).unwrap()
}

/// An app instance logged in as `email`.
pub fn client(email: &str) -> SesameClient {
    let client = anonymous();
    let status = login(&client, &raw_apikey(email)).status();
    assert_eq!(status, Status::SeeOther, "could not log in as {}", email);
    client
}

pub fn login<'c>(client: &'c SesameClient, key: &str) -> LocalResponse<'c> {
    post(client, "/apikey/check", format!("key={}", key))
}

/// POST a urlencoded form.
pub fn post<'c, U: AsRef<str>, B: Into<String>>(
    client: &'c SesameClient,
    uri: U,
    body: B,
) -> LocalResponse<'c> {
    client
        .post(uri.as_ref().to_string())
        .header(ContentType::Form)
        .body(body.into())
        .dispatch()
}

/// The body of a response that is expected to have rendered.
pub fn ok_body(response: LocalResponse<'_>) -> String {
    assert_eq!(response.status(), Status::Ok, "expected a rendered page");
    response.into_string().unwrap()
}

/// A urlencoded answer submission: one answer per question id.
pub fn answers_body(question_ids: &[u64], answer: &str) -> String {
    question_ids
        .iter()
        .map(|id| format!("answers.{}={}", id, urlencode(answer)))
        .collect::<Vec<String>>()
        .join("&")
}

pub fn urlencode(value: &str) -> String {
    value
        .chars()
        .map(|c| match c {
            ' ' => String::from("+"),
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '@' => c.to_string(),
            other => format!("%{:02X}", other as u32),
        })
        .collect()
}

/// Register a user and return their API key.
pub fn register(client: &SesameClient, email: &str) -> String {
    let response = post(
        client,
        "/apikey/generate",
        format!("email={}&consent=true", urlencode(email)),
    );
    assert_eq!(
        response.status(),
        Status::Ok,
        "registration failed for {}",
        email
    );
    raw_apikey(email)
}

/// Build the test database once per test binary:
///   * an admin and five students, all consenting,
///   * three lectures with two questions each,
///   * corinn presenting lecture 1, artem presenting lecture 2, and lecture 3
///     with no presenter at all.
pub fn seed() {
    static SEEDED: OnceLock<()> = OnceLock::new();
    SEEDED.get_or_init(|| {
        let client = SesameClient::tracked(make_rocket(config(true))).unwrap();

        register(&client, ADMIN);
        for student in STUDENTS {
            register(&client, student);
        }

        // The rest of the seeding goes through the admin routes.
        login(&client, &raw_apikey(ADMIN));

        for lecture in 1..=3u64 {
            post(
                &client,
                "/admin/lec/add",
                format!("lec_id={}&lec_label=Lecture+{}", lecture, lecture),
            );
            for question in 1..=2u64 {
                post(
                    &client,
                    format!("/admin/lec/{}", lecture),
                    format!("q_prompt=Lecture+{}+question+{}", lecture, question),
                );
            }
        }

        for (lecture, presenter) in [(1, CORINN), (2, ARTEM), (3, "")] {
            post(
                &client,
                format!("/admin/lec/edit/{}", lecture),
                format!(
                    "lec_name=Lecture+{}&lec_presenters={}",
                    lecture,
                    urlencode(presenter)
                ),
            );
        }
    });
}
