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
use crate::guards::admin::Admin;
use crate::policies::{AnswerAccessPolicy, ContextData, UserEmailPolicy};
use serde::Serialize;
use sesame::SesameType;

// TODO (allen): do we need PCon's for context to our pages? and what kind of policy should they have?
#[derive(PConRender)]
pub struct AnswersRender {
    pub lec_id: PCon<u8, NoPolicy>,
    pub answers: PCon<Vec<AnswerRowOut>, AnyPolicy>,
}

#[get("/<num>")]
pub(crate) fn composed_answers(
    _admin: Admin,
    num: PCon<u8, NoPolicy>,
    backend: &State<Arc<Mutex<MySqlBackend>>>,
    context: Context<ContextData>,
) -> PConTemplate {
    let mut bg = backend.lock().unwrap();
    let key = num.clone().into_pcon::<u64, NoPolicy>();
    let res = bg.query_answers(
        "lec",
        (key,),
        context.clone(),
    );
    drop(bg);

    let answers: Vec<AnswerRow> = res
        .into_iter()
        .map(|a| AnswerRow {
            question_id: a.question_id,
            email: a.email,
            lec: a.lec,
            answer: a.answer,
            submitted_at: a.submitted_at.into_verified(VerifiedRegion::new(
                |v: NaiveDateTime| v.format("%Y-%m-%d %H:%M:%S").to_string(),
            )),
        })
        .collect();

    let outer_box_answers = fold::<dyn AnyPolicyDyn, _>(answers).unwrap();

    let ctx = AnswersRender {lec_id: num, answers: outer_box_answers};
    PConTemplate::render("admin/answers", &ctx, context).unwrap()
}

#[derive(PConRender, SesameType)]
#[sesame_out_type(to_derive = [PConRender, Clone, Serialize])]
pub struct AnswerRow {
    pub question_id: PCon<u64, AnswerAccessPolicy>,
    pub email: PCon<String, UserEmailPolicy>,
    pub lec: PCon<u64, AnswerAccessPolicy>,
    pub answer: PCon<String, AnswerAccessPolicy>,
    pub submitted_at: PCon<String, AnswerAccessPolicy>,
}
