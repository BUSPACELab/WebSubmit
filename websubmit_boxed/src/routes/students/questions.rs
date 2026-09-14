use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::Local;
use rocket::State;
use sesame::context::Context;
use sesame::critical::{execute_critical, CriticalRegion, Signature};
use sesame::pcon::PCon;
use sesame::policy::{AnyPolicy, AnyPolicyDyn, NoPolicy};
use sesame::verified::{execute_verified, VerifiedRegion};
use sesame_rocket::render::PConRender;
use sesame_rocket::rocket::{get, post, FromPConForm, PConForm, PConRedirect, PConTemplate};

use crate::config::Config;
use crate::db::MySqlBackend;
use crate::email;
use crate::guards::apikey::ApiKey;
use crate::policies::ContextData;
use serde::Serialize;

// TODO (allen): is this NoPolicy because it came from the user and we're going to write it (not for reading yet?)
#[derive(Debug, FromPConForm)]
pub(crate) struct LectureAnswersForm {
    answers: HashMap<u64, PCon<String, NoPolicy>>,
}

// TODO (allen): do we need PCon's for context to our pages?
#[derive(PConRender)]
pub(crate) struct QuestionsRender {
    pub lec_id: PCon<u8, NoPolicy>,
    pub questions: PCon<Vec<QuestionJoinAnswerRow>, AnyPolicy>,
}

#[get("/<num>")]
pub(crate) fn questions(
    apikey: ApiKey,
    num: PCon<u8, NoPolicy>,
    backend: &State<Arc<Mutex<MySqlBackend>>>,
    context: Context<ContextData>,
) -> PConTemplate {
    let mut bg = backend.lock().unwrap();
    let key = num.clone().into_pcon::<u64, NoPolicy>();

    // The view already left-joins this user's answers onto the lecture's
    // questions, so unanswered questions come back with a NULL answer.
    let rows = bg.query_questions_with_answers(
        ("lecture_id", "user_email"),
        (key, apikey.user),
        context.clone(),
    );
    drop(bg);

    let rows: Vec<_> = rows
        .into_iter()
        .map(|r| (r.id, r.question_number, r.question, r.answer))
        .collect();

    let questions: PCon<Vec<QuestionJoinAnswerRow>, AnyPolicy> =
        execute_verified::<dyn AnyPolicyDyn, _, _, _>(
            rows,
            VerifiedRegion::new(
                |rows: Vec<(u64, u64, String, Option<String>)>| {
                    let mut rows: Vec<QuestionJoinAnswerRow> = rows
                        .into_iter()
                        .map(|(id, question_num, prompt, answer)| QuestionJoinAnswerRow {
                            id,
                            question_num,
                            prompt,
                            answer,
                        })
                        .collect();
                    rows.sort_by_key(|q| q.id);
                    rows
                },
            ),
        )
        .unwrap();

    let ctx = QuestionsRender {lec_id: num, questions: questions};

    PConTemplate::render("students/questions", &ctx, context).unwrap()
}

#[post("/<num>", data = "<data>")]
pub(crate) fn questions_submit(
    apikey: ApiKey,
    num: PCon<u8, NoPolicy>,
    data: PConForm<LectureAnswersForm>,
    backend: &State<Arc<Mutex<MySqlBackend>>>,
    config: &State<Config>,
    context: Context<ContextData>,
) -> PConRedirect {
    let num = num.into_pcon::<u64, NoPolicy>();
    let ts: mysql::Value = Local::now().naive_local().into();

    let mut bg = backend.lock().unwrap();
    for (id, answer) in &data.answers {
        // Answers use a synthesised primary key of "{email}-{question_id}".
        let question_id = *id;
        let answer_id = apikey
            .user
            .clone()
            .into_verified(VerifiedRegion::new(move |email: String| {
                format!("{}-{}", email, question_id)
            }));
        bg.replace(
            "answers",
            (
                answer_id,
                apikey.user.clone(),
                num.clone(),
                PCon::new(question_id, NoPolicy {}),
                answer.clone(),
                PCon::new(ts.clone(), NoPolicy {}),
            ),
            context.clone(),
        );
    }

    if config.send_emails {
        let data = (data.answers.clone(), num, apikey.user);
        let result = execute_critical(
            data,
            context,
            CriticalRegion::new(
                |(answers, num, user): (HashMap<u64, String>, u64, String), _| {
                    let answer_log = format!(
                        "{}",
                        answers
                            .iter()
                            .map(|(i, t)| format!("Question {}:\n{}", i, t))
                            .collect::<Vec<String>>()
                            .join("\n-----\n"),
                    );

                    // config is component of the context -> has passed policy check at this point
                    let recipients = config.staff.clone();

                    email::send(
                        bg.log.clone(),
                        config.inner(),
                        Some(user),
                        recipients,
                        format!("{} meeting {} questions", config.class, num,),
                        answer_log,
                    )
                    .expect("failed to send email")
                },
                Signature{username: "corinnt", signature: "TODO"}),
            (),
        );
        result.unwrap();
    }
    drop(bg);

    PConRedirect::to2("/leclist")
}

// The answer form's rows: plain values, wrapped as one PCon by the caller.
#[derive(PConRender, Clone, Serialize)]
pub(crate) struct QuestionJoinAnswerRow {
    pub id: u64,
    pub question_num: u64,
    pub prompt: String,
    pub answer: Option<String>,
}
