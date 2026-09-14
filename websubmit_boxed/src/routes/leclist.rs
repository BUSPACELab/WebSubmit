use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rocket::State;
use sesame::context::Context;
use sesame::pcon::PCon;
use sesame::policy::{AnyPolicy, AnyPolicyDyn};
use sesame::verified::{execute_verified, VerifiedRegion};
use sesame_rocket::render::PConRender;
use sesame_rocket::rocket::{get, PConTemplate};

use crate::config::Config;
use crate::db::MySqlBackend;
use crate::guards::apikey::ApiKey;
use crate::policies::{ContextData, UserEmailPolicy};
use serde::Serialize;

#[derive(PConRender)]
struct LectureListRender {
    admin: PCon<bool, UserEmailPolicy>,
    lectures: PCon<Vec<AggregateLectureRow>, AnyPolicy>,
}

#[get("/")]
pub(crate) fn leclist(
    apikey: ApiKey,
    backend: &State<Arc<Mutex<MySqlBackend>>>,
    config: &State<Config>,
    context: Context<ContextData>,
) -> PConTemplate {
    let mut bg = backend.lock().unwrap();
    let res = bg.query_lectures_with_question_counts((), (), context.clone());
    // The viewer's own answers, used for the answered counts below.
    let answers_res = bg.query_answers(
        "email",
        (apikey.user.clone(),),
        context.clone(),
    );
    drop(bg);

    let admin: PCon<bool, UserEmailPolicy> = apikey.user.into_verified(
        VerifiedRegion::new(|email| {
            config.admins.contains(&email)
        })
    );

    // The view carries no policy.
    let lecture_meta: Vec<(u64, String, u64)> = res
        .into_iter()
        .map(|l| {
            (
                l.id.discard_box(),
                l.label.discard_box(),
                l.U_c.discard_box(),
            )
        })
        .collect();

    // Answers are policy protected, so count them inside a verified region; the
    // resulting list stays under the answers' policy.
    let submissions: Vec<_> = answers_res.into_iter().map(|a| (a.lec, a.answer)).collect();
    let lectures: PCon<Vec<AggregateLectureRow>, AnyPolicy> =
        execute_verified::<dyn AnyPolicyDyn, _, _, _>(
            submissions,
            VerifiedRegion::new(move |rows: Vec<(u64, String)>| {
                let mut answered: HashMap<u64, u64> = HashMap::new();
                for (lec, answer) in rows {
                    if answer.trim().is_empty() {
                        continue;
                    }
                    *answered.entry(lec).or_insert(0) += 1;
                }
                lecture_meta
                    .iter()
                    .map(|(id, label, num_qs)| AggregateLectureRow {
                        id: *id,
                        label: label.clone(),
                        num_qs: *num_qs,
                        num_answered: *answered.get(id).unwrap_or(&0),
                    })
                    .collect::<Vec<AggregateLectureRow>>()
            }),
        )
        .unwrap();

    let ctx = LectureListRender {admin, lectures};

    PConTemplate::render("leclist", &ctx, context).unwrap()
}

#[derive(Serialize)]
pub(crate) struct AggregateLectureRow {
    pub id: u64,
    pub label: String,
    pub num_qs: u64,
    pub num_answered: u64,
}
