use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use rocket::State;
use sesame::context::Context;
use sesame::pcon::PCon;
use sesame::policy::{AnyPolicy, AnyPolicyDyn, NoPolicy};
use sesame::verified::{execute_verified, VerifiedRegion};
use sesame_rocket::render::PConRender;
use sesame_rocket::rocket::{get, PConTemplate};

use crate::config::Config;
use crate::db::MySqlBackend;
use crate::guards::admin::Admin;
use crate::policies::{ContextData, UserEmailPolicy};
use serde::Serialize;

#[derive(PConRender)]
struct UserListRender {
    users: Vec<UserRow>,
}

#[derive(PConRender)]
struct GradingRender {
    lectures_count: u64,
    lectures: Vec<AggregateLectureRow>,
    users: PCon<Vec<AggregateUserRow>, AnyPolicy>,
}

#[get("/")]
pub(crate) fn get_registered_users(
    _adm: Admin,
    backend: &State<Arc<Mutex<MySqlBackend>>>,
    config: &State<Config>,
    context: Context<ContextData>,
) -> PConTemplate {
    let mut bg = backend.lock().unwrap();
    let res = bg.query_users((), (), context.clone());
    drop(bg);

    let users = res
        .into_iter()
        .map(|user| UserRow {
            is_admin: user
                .email
                .clone()
                .into_verified(VerifiedRegion::new(|id| config.admins.contains(&id))),
            email: user.email,
            consent: user.consent,
        })
        .collect();

    let ctx = UserListRender {users: users};
    PConTemplate::render("admin/users/list", &ctx, context).unwrap()
}

#[get("/")]
pub(crate) fn grading(
    _adm: Admin,
    backend: &State<Arc<Mutex<MySqlBackend>>>,
    context: Context<ContextData>,
) -> PConTemplate {
    let mut bg = backend.lock().unwrap();
    let lectures_res = bg.query_lectures((), (), context.clone());
    let questions_res = bg.query_questions((), (), context.clone());
    let users_res = bg.query_users((), (), context.clone());
    let answers_res = bg.query_answers((), (), context.clone());
    drop(bg);

    // Lectures: the lectures table carries no policy.
    let mut lectures_map = BTreeMap::new();
    for lecture in lectures_res {
        lectures_map.insert(lecture.id.discard_box(), lecture.label.discard_box());
    }
    let mut lectures: Vec<AggregateLectureRow> = lectures_map
        .into_iter()
        .map(|(id, label)| AggregateLectureRow {
            id,
            label,
            num_qs: 0,
        })
        .collect();
    let lectures_count = lectures.len() as u64;

    // Questions per lecture.
    for question in questions_res {
        let lec: u64 = question.lecture_id.discard_box();
        for lecture in &mut lectures {
            if lecture.id == lec {
                lecture.num_qs += 1;
            }
        }
    }

    // Both users.email and the answers are policy protected, so resolve them
    // together inside a verified region; the report inherits both policies.
    let submissions: Vec<_> = answers_res
        .into_iter()
        .map(|a| (a.email, a.lec, a.answer))
        .collect();
    let user_emails: Vec<_> = users_res.into_iter().map(|u| u.email).collect();
    let lecture_ids: Vec<(u64, u64)> = lectures.iter().map(|l| (l.id, l.num_qs)).collect();
    let users: PCon<Vec<AggregateUserRow>, AnyPolicy> =
        execute_verified::<dyn AnyPolicyDyn, _, _, _>(
            (submissions, user_emails),
            VerifiedRegion::new(
                move |(rows, report_emails): (Vec<(String, u64, String)>, Vec<String>)| {
                let mut users_map: HashMap<String, HashMap<u64, u64>> = report_emails
                    .iter()
                    .map(|email| {
                        (
                            email.clone(),
                            lecture_ids.iter().map(|(id, _)| (*id, 0u64)).collect(),
                        )
                    })
                    .collect();

                for (email, lec, answer) in rows {
                    if answer.trim().len() < 10 {
                        continue;
                    }
                    if let Some(counts) = users_map.get_mut(&email) {
                        if let Some(count) = counts.get_mut(&lec) {
                            *count += 1;
                        }
                    }
                }

                report_emails
                    .iter()
                    .map(|email| {
                        let counts = users_map.get(email).unwrap();
                        let mut lectures = Vec::new();
                        let mut total = 0.0;
                        for (id, questions) in &lecture_ids {
                            let count = *counts.get(id).unwrap();
                            if *questions > 0 {
                                total += count as f64 / *questions as f64;
                            }
                            lectures.push(count);
                        }
                        AggregateUserRow {
                            email: email.clone(),
                            total,
                            lectures,
                        }
                    })
                    .collect::<Vec<AggregateUserRow>>()
                },
            ),
        )
        .unwrap();

    let ctx = GradingRender {lectures_count, lectures, users};

    PConTemplate::render("admin/users/grading", &ctx, context).unwrap()
}

#[derive(PConRender, Clone)]
pub(crate) struct UserRow {
    pub email: PCon<String, UserEmailPolicy>,
    pub is_admin: PCon<bool, UserEmailPolicy>,
    pub consent: PCon<bool, NoPolicy>,
    // apikey is omitted: QueryableOnly forbids rendering it.
}

#[derive(Serialize)]
pub(crate) struct AggregateUserRow {
    pub email: String,
    pub total: f64,
    pub lectures: Vec<u64>,
}

/// Lectures on the grading report: the question count is what the report
/// divides by. Unlike the leclist row this carries no answered count, since the
/// per-user counts live in AggregateUserRow.
#[derive(Serialize)]
pub(crate) struct AggregateLectureRow {
    pub id: u64,
    pub label: String,
    pub num_qs: u64,
}
