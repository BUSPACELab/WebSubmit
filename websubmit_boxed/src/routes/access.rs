use std::sync::{Arc, Mutex};

use chrono::naive::NaiveDateTime;
use rocket::State;
use sesame::context::Context;
use sesame::critical::{execute_critical, CriticalRegion, Signature};
use sesame::fold::fold;
use sesame::pcon::PCon;
use sesame::policy::{AnyPolicy, AnyPolicyDyn, NoPolicy};
use sesame::verified::VerifiedRegion;
use sesame::SesameType;
use sesame_rocket::render::PConRender;
use sesame_rocket::rocket::{get, PConTemplate};
use serde::Serialize;

use crate::db::MySqlBackend;
use crate::guards::apikey::ApiKey;
use crate::policies::{AnswerAccessPolicy, ContextData, UserEmailPolicy};

#[get("/")]
pub(crate) fn access(
    apikey: ApiKey,
    backend: &State<Arc<Mutex<MySqlBackend>>>,
    context: Context<ContextData>,
) -> PConTemplate {
    let mut bg = backend.lock().unwrap();

    // GDPR GET cannot be a prepared statement (K9db rejects it outright), so
    // the email has to leave its PCon before it can be embedded as a literal.
    // This is the same kind of raw-value extraction questions_submit uses to
    // hand an email to the SMTP client: a critical region, checked against
    // this request's own UserEmailPolicy (which passes here because the
    // requester is always asking about themselves). Everything gdpr_get
    // fetches runs inside the region, so nothing is returned out of it but
    // fully policy-tagged rows.
    let (users, answers, presenters) = execute_critical(
        apikey.user.clone(),
        context.clone(),
        CriticalRegion::new(
            move |email: String, _: ()| bg.gdpr_get(&email),
            Signature {
                username: "babman",
                signature: "",
            },
        ),
        (),
    )
    .unwrap();

    let users: Vec<GdprUserRow> = users
        .into_iter()
        .map(|u| GdprUserRow {
            email: u.email,
            is_admin: u.is_admin,
            consent: u.consent,
        })
        .collect();
    let answers: Vec<GdprAnswerRow> = answers
        .into_iter()
        .map(|a| GdprAnswerRow {
            id: a.id,
            lec: a.lec,
            question_id: a.question_id,
            answer: a.answer,
            submitted_at: a.submitted_at.into_verified(VerifiedRegion::new(
                |v: NaiveDateTime| v.format("%Y-%m-%d %H:%M:%S").to_string(),
            )),
        })
        .collect();
    let presenters: Vec<GdprPresenterRow> = presenters
        .into_iter()
        .map(|p| GdprPresenterRow {
            lecture_id: p.lecture_id,
            email: p.email,
        })
        .collect();

    let ctx = AccessRender {
        users: fold::<dyn AnyPolicyDyn, _>(users).unwrap(),
        answers: fold::<dyn AnyPolicyDyn, _>(answers).unwrap(),
        presenters: fold::<dyn AnyPolicyDyn, _>(presenters).unwrap(),
    };

    PConTemplate::render("access", &ctx, context).unwrap()
}

#[derive(PConRender)]
struct AccessRender {
    users: PCon<Vec<GdprUserRowOut>, AnyPolicy>,
    answers: PCon<Vec<GdprAnswerRowOut>, AnyPolicy>,
    presenters: PCon<Vec<GdprPresenterRowOut>, AnyPolicy>,
}

// The `users` row: the API key is deliberately never read here (QueryableOnly
// forbids rendering it), so the template hard-codes a placeholder for it
// instead of showing a real value.
#[derive(PConRender, SesameType)]
#[sesame_out_type(to_derive = [PConRender, Clone, Serialize])]
struct GdprUserRow {
    email: PCon<String, UserEmailPolicy>,
    is_admin: PCon<bool, NoPolicy>,
    consent: PCon<bool, NoPolicy>,
}

#[derive(PConRender, SesameType)]
#[sesame_out_type(to_derive = [PConRender, Clone, Serialize])]
struct GdprAnswerRow {
    id: PCon<String, UserEmailPolicy>,
    lec: PCon<u64, AnswerAccessPolicy>,
    question_id: PCon<u64, AnswerAccessPolicy>,
    answer: PCon<String, AnswerAccessPolicy>,
    submitted_at: PCon<String, AnswerAccessPolicy>,
}

#[derive(PConRender, SesameType)]
#[sesame_out_type(to_derive = [PConRender, Clone, Serialize])]
struct GdprPresenterRow {
    lecture_id: PCon<u64, NoPolicy>,
    email: PCon<String, UserEmailPolicy>,
}
