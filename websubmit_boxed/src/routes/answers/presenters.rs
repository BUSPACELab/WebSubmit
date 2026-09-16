use std::sync::{Arc, Mutex};

use chrono::naive::NaiveDateTime;
use rocket::State;
use sesame::context::Context;
use sesame::fold::fold;
use sesame::pcon::PCon;
use sesame::policy::{AnyPolicy, AnyPolicyDyn, NoPolicy};
use sesame::verified::VerifiedRegion;
use sesame_rocket::render::PConRender;
use sesame_rocket::rocket::{get, PConTemplate};

use crate::db::MySqlBackend;
use crate::guards::apikey::ApiKey;
use crate::policies::{AnswerAccessPolicy, ContextData};
use serde::Serialize;
use sesame::SesameType;

#[derive(PConRender)]
pub struct AnonymousAnswersRender {
    pub lec_id: PCon<u8, NoPolicy>,
    pub answers: PCon<Vec<AnonymousAnswerRowOut>, AnyPolicy>,
}

#[get("/presenters/<num>")]
pub(crate) fn answers_for_presenters(
    num: PCon<u8, NoPolicy>,
    apikey: ApiKey,
    backend: &State<Arc<Mutex<MySqlBackend>>>,
    context: Context<ContextData>,
) -> PConTemplate {
    let key = num.clone().into_pcon::<u64, NoPolicy>();

    let mut bg = backend.lock().unwrap();
    let res = bg.query_answers(
        "lec",
        (key,),
        context.clone(),
    );
    drop(bg);

    // The email column is never read here, so presenters see anonymous answers.
    let answers: Vec<AnonymousAnswerRow> = res
        .into_iter()
        .map(|a| AnonymousAnswerRow {
            question_id: a.question_id,
            lec: a.lec,
            answer: a.answer,
            submitted_at: a.submitted_at.into_verified(VerifiedRegion::new(
                |v: NaiveDateTime| v.format("%Y-%m-%d %H:%M:%S").to_string(),
            )),
        })
        .collect();

    let outer_box_answers = fold::<dyn AnyPolicyDyn, _>(answers).unwrap();

    let ctx = AnonymousAnswersRender {lec_id: num, answers: outer_box_answers};
    PConTemplate::render("answers/presenters", &ctx, context).unwrap()
}

#[derive(PConRender, SesameType)]
#[sesame_out_type(to_derive = [PConRender, Clone, Serialize])]
pub struct AnonymousAnswerRow {
    pub question_id: PCon<u64, AnswerAccessPolicy>,
    pub lec: PCon<u64, AnswerAccessPolicy>,
    pub answer: PCon<String, AnswerAccessPolicy>,
    pub submitted_at: PCon<String, AnswerAccessPolicy>,
}
