use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::naive::NaiveDateTime;
use chrono::Local;
use mysql::Value;
use rocket::State;

use crate::admin::Admin;
use sesame::pcon::PCon;
use sesame_rocket::render::PConRender;
use sesame::context::Context;
use sesame_mysql::{from_value, PConRow};
use sesame::fold::fold;
use sesame::critical::{execute_critical, CriticalRegion, Signature};
use sesame::policy::{AnyPolicyDyn, NoPolicy};
use sesame::verified::{execute_verified, VerifiedRegion};
use sesame_rocket::rocket::{get, post, PConForm, PConRedirect, PConTemplate, FromPConForm};
use sesame::SesameType;

use crate::apikey::ApiKey;
use crate::backend::MySqlBackend;
use crate::config::Config;
use crate::email;
use crate::helpers::{left_join, JoinIdx};
use crate::policies::{AnswerAccessPolicy, ContextData};

// TODO (allen): is this NoPolicy because it came from the user and we're going to write it (not for reading yet?)
#[derive(Debug, FromPConForm)]
pub(crate) struct LectureQuestionSubmission {
    answers: HashMap<u64, PCon<String, NoPolicy>>,
}

// TODO (allen): these are NoPolicy because not sensitive information? but answer could be right?
#[derive(PConRender, Clone)]
pub(crate) struct LectureQuestion {
    pub id: PCon<u64, NoPolicy>,
    pub prompt: PCon<String, NoPolicy>,
    pub answer: PCon<Option<String>, NoPolicy>,
}

// TODO (allen): do we need PCon's for context to our pages?
#[derive(PConRender)]
pub(crate) struct LectureQuestionsContext {
    pub lec_id: PCon<u8, NoPolicy>,
    pub questions: Vec<LectureQuestion>,
    pub parent: String,
}

#[derive(PConRender, Clone, SesameType)]
#[sesame_out_type(to_derive = [PConRender, Clone, Serialize])]
//#[derive(PConRender, Clone)]
pub struct LectureAnswer {
    pub id: PCon<u64, AnswerAccessPolicy>,
    pub user: PCon<String, AnswerAccessPolicy>,
    pub answer: PCon<String, AnswerAccessPolicy>,
    pub time: PCon<String, AnswerAccessPolicy>,
    pub grade: PCon<u64, AnswerAccessPolicy>,
}

// TODO (allen): do we need PCon's for context to our pages? and what kind of policy should they have?
#[derive(PConRender)]
pub struct LectureAnswersContext {
    pub lec_id: PCon<u8, NoPolicy>,
    pub answers: PCon<Vec<LectureAnswerOut>, AnswerAccessPolicy>,
    pub parent: String,
}

#[derive(PConRender)]
pub struct NaiveLectureAnswersContext {
    pub lec_id: PCon<u8, NoPolicy>,
    pub answers: Vec<LectureAnswer>,
    pub parent: String,
}

// TODO (allen): these are NoPolicy because not sensitive user information?
#[derive(PConRender, SesameType)]
#[sesame_out_type(to_derive = [PConRender, Clone])]
struct LectureListEntry {
    id: PCon<u64, NoPolicy>,
    label: PCon<String, NoPolicy>,
    num_qs: PCon<u64, NoPolicy>,
    num_answered: u64,
}

// TODO (allen): do we need PCon's for context to our pages? and what kind of policy should they have?
#[derive(PConRender)]
struct LectureListContext {
    admin: PCon<bool, NoPolicy>,
    lectures: Vec<LectureListEntry>,
    parent: String,
}

#[get("/")]
pub(crate) fn leclist(
    apikey: ApiKey,
    backend: &State<Arc<Mutex<MySqlBackend>>>,
    config: &State<Config>,
    context: Context<ContextData>,
) -> PConTemplate {
    let mut bg = backend.lock().unwrap();
    let res = bg.prep_exec(
        "SELECT lectures.id, lectures.label, lec_qcount.qcount \
         FROM lectures \
         LEFT JOIN lec_qcount ON (lectures.id = lec_qcount.lec)",
        (),
        context.clone(),
    );
    drop(bg);

    let admin: PCon<bool, NoPolicy> = apikey.user.into_verified(VerifiedRegion::new(|email| {
        config.admins.contains(&email)
    }));

    let lecs: Vec<LectureListEntry> = res
        .into_iter()
        .map(|r: PConRow| LectureListEntry {
            id: from_value(r.get(0).unwrap()).unwrap(),
            label: from_value(r.get(1).unwrap()).unwrap(),
            num_qs: r
                .get(2)
                .unwrap()
                .specialize_policy()
                .unwrap()
                .into_verified(VerifiedRegion::new(|v| match v {
                    Value::NULL => 0u64,
                    v => mysql::from_value(v),
                })),
            num_answered: 0u64,
        })
        .collect();

    let ctx = LectureListContext {
        admin,
        lectures: lecs,
        parent: "layout".into(),
    };

    PConTemplate::render("leclist", &ctx, context).unwrap()
}

#[get("/naive/<num>")]
pub(crate) fn naive_answers(
    _admin: Admin,
    num: PCon<u8, NoPolicy>,
    backend: &State<Arc<Mutex<MySqlBackend>>>,
    context: Context<ContextData>,
) -> PConTemplate {
    let mut bg = backend.lock().unwrap();
    let key = num.clone().into_pcon::<u64, NoPolicy>();
    let res = bg.prep_exec(
        "SELECT * FROM answers WHERE lec = ?",
        (key,),
        context.clone(),
    );
    drop(bg);

    // Wraps incoming column data in LectureAnswer format
    let answers: Vec<LectureAnswer> = res
        .into_iter()
        .map(|r| LectureAnswer {
            id: from_value(r.get(2).unwrap()).unwrap(),
            user: from_value(r.get(0).unwrap()).unwrap(),
            answer: from_value(r.get(3).unwrap()).unwrap(),
            time: from_value(r.get(4).unwrap())
                .unwrap()
                .into_verified(VerifiedRegion::new(|v: NaiveDateTime| {
                    v.format("%Y-%m-%d %H:%M:%S").to_string()
                })),
            grade: from_value(r.get(5).unwrap()).unwrap(),
        })
        .collect();

    let ctx = NaiveLectureAnswersContext {
        lec_id: num,
        answers,
        parent: "layout".into(),
    };
    PConTemplate::render("answers", &ctx, context).unwrap()
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
    let res = bg.prep_exec(
        "SELECT * FROM answers WHERE lec = ?",
        (key,),
        context.clone(),
    );
    drop(bg);

    // Wraps incoming column data in LectureAnswer format
    let answers: Vec<LectureAnswer> = res
        .into_iter()
        .map(|r| LectureAnswer {
            id: from_value(r.get(2).unwrap()).unwrap(),
            user: from_value(r.get(0).unwrap()).unwrap(),
            answer: from_value(r.get(3).unwrap()).unwrap(),
            time: from_value(r.get(4).unwrap())
                .unwrap()
                .into_verified(VerifiedRegion::new(|v: NaiveDateTime| {
                    v.format("%Y-%m-%d %H:%M:%S").to_string()
                })),
            grade: from_value(r.get(5).unwrap()).unwrap(),
        })
        .collect();

    let outer_box_answers = fold::<dyn AnyPolicyDyn, _>(answers)
        .unwrap()
        .specialize_policy::<AnswerAccessPolicy>()
        .unwrap();

    let ctx = LectureAnswersContext {
        lec_id: num,
        answers: outer_box_answers,
        parent: "layout".into(),
    };
    PConTemplate::render("answers", &ctx, context).unwrap()
}

#[get("/discussion_leaders/<num>")]
pub(crate) fn answers_for_discussion_leaders(
    num: PCon<u8, NoPolicy>,
    apikey: ApiKey,
    backend: &State<Arc<Mutex<MySqlBackend>>>,
    context: Context<ContextData>,
) -> PConTemplate {
    let key = num.clone().into_pcon::<u64, NoPolicy>();

    let is_discussion_leader = {
        let mut bg = backend.lock().unwrap();
        let vec = bg.prep_exec(
            "SELECT * FROM discussion_leaders WHERE lec = ? AND email = ?",
            (key.clone(), apikey.user),
            context.clone(),
        );
        vec.len() > 0
    };

    if !is_discussion_leader {
        panic!()
    }

    let mut bg = backend.lock().unwrap();
    let res = bg.prep_exec(
        "SELECT * FROM answers WHERE lec = ?",
        (key,),
        context.clone(),
    );
    drop(bg);

    // Wraps incoming column data in LectureAnswer format
    let answers: Vec<LectureAnswer> = res
        .into_iter()
        .map(|r| LectureAnswer {
            id: from_value(r.get(2).unwrap()).unwrap(),
            user: from_value(r.get(0).unwrap()).unwrap(),
            answer: from_value(r.get(3).unwrap()).unwrap(),
            time: from_value(r.get(4).unwrap())
                .unwrap()
                .into_verified(VerifiedRegion::new(|v: NaiveDateTime| {
                    v.format("%Y-%m-%d %H:%M:%S").to_string()
                })),
            grade: from_value(r.get(5).unwrap()).unwrap(),
        })
        .collect();

    let outer_box_answers = fold::<dyn AnyPolicyDyn, _>(answers)
        .unwrap()
        .specialize_policy::<AnswerAccessPolicy>()
        .unwrap();

    let ctx = LectureAnswersContext {
        lec_id: num,
        answers: outer_box_answers,
        parent: "layout".into(),
    };
    PConTemplate::render("answers", &ctx, context).unwrap()
}

#[get("/discussion_leaders/naive/<num>")]
pub(crate) fn answers_for_discussion_leaders_naive(
    num: PCon<u8, NoPolicy>,
    apikey: ApiKey,
    backend: &State<Arc<Mutex<MySqlBackend>>>,
    context: Context<ContextData>,
) -> PConTemplate {
    let key = num.clone().into_pcon::<u64, NoPolicy>();

    let is_discussion_leader = {
        let mut bg = backend.lock().unwrap();
        let vec = bg.prep_exec(
            "SELECT * FROM discussion_leaders WHERE lec = ? AND email = ?",
            (key.clone(), apikey.user),
            context.clone(),
        );
        vec.len() > 0
    };

    if !is_discussion_leader {
        panic!()
    }

    let mut bg = backend.lock().unwrap();
    let res = bg.prep_exec(
        "SELECT * FROM answers WHERE lec = ?",
        (key,),
        context.clone(),
    );
    drop(bg);

    // Wraps incoming column data in LectureAnswer format
    let answers: Vec<LectureAnswer> = res
        .into_iter()
        .map(|r| LectureAnswer {
            id: from_value(r.get(2).unwrap()).unwrap(),
            user: from_value(r.get(0).unwrap()).unwrap(),
            answer: from_value(r.get(3).unwrap()).unwrap(),
            time: from_value(r.get(4).unwrap())
                .unwrap()
                .into_verified(VerifiedRegion::new(|v: NaiveDateTime| {
                    v.format("%Y-%m-%d %H:%M:%S").to_string()
                })),
            grade: from_value(r.get(5).unwrap()).unwrap(),
        })
        .collect();

    let ctx = NaiveLectureAnswersContext {
        lec_id: num,
        answers,
        parent: "layout".into(),
    };
    PConTemplate::render("answers", &ctx, context).unwrap()
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

    let answers_result = bg.prep_exec(
        "SELECT answers.* FROM answers WHERE answers.lec = ? AND answers.email = ?",
        (key.clone(), apikey.user),
        context.clone(),
    );
    let questions_result = bg.prep_exec(
        "SELECT * FROM questions WHERE lec = ?",
        (key,),
        context.clone(),
    );
    drop(bg);

    // left_join operates on whole rows, so flatten PConRow -> Vec<PConValue>.
    let answers_result = answers_result
        .into_iter()
        .map(|r| r.unwrap())
        .collect::<Vec<_>>();
    let questions_result = questions_result
        .into_iter()
        .map(|r| r.unwrap())
        .collect::<Vec<_>>();

    let questions: PCon<Vec<Vec<Value>>, NoPolicy> = execute_verified::<dyn AnyPolicyDyn, _, _, _>(
        (questions_result, answers_result),
        VerifiedRegion::new(|(questions, answers)| {
            let mut questions = left_join(
                questions,
                answers,
                1,
                2,
                vec![JoinIdx::Left(1), JoinIdx::Left(2), JoinIdx::Right(3)],
            );
            questions.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap());
            questions
        }),
    )
    .unwrap()
    .specialize_policy::<NoPolicy>()
    .unwrap();

    let questions: Vec<PCon<Vec<Value>, NoPolicy>> = questions.fold_in();
    let questions = questions
        .into_iter()
        .map(|r: PCon<Vec<Value>, NoPolicy>| {
            let r: Vec<PCon<Value, NoPolicy>> = r.fold_in();
            // Policy is already specialized here, so convert the value in place
            // rather than going through from_value (which expects AnyPolicy).
            LectureQuestion {
                id: r[0]
                    .clone()
                    .into_verified(VerifiedRegion::new(mysql::from_value)),
                prompt: r[1]
                    .clone()
                    .into_verified(VerifiedRegion::new(mysql::from_value)),
                answer: r[2]
                    .clone()
                    .into_verified(VerifiedRegion::new(mysql::from_value)),
            }
        })
        .collect();
    let ctx = LectureQuestionsContext {
        lec_id: num,
        questions: questions,
        parent: "layout".into(),
    };

    PConTemplate::render("questions", &ctx, context).unwrap()
}

#[post("/<num>", data = "<data>")]
pub(crate) fn questions_submit(
    apikey: ApiKey,
    num: PCon<u8, NoPolicy>,
    data: PConForm<LectureQuestionSubmission>,
    backend: &State<Arc<Mutex<MySqlBackend>>>,
    config: &State<Config>,
    context: Context<ContextData>,
) -> PConRedirect {
    let num = num.into_pcon::<u64, NoPolicy>();
    let ts: mysql::Value = Local::now().naive_local().into();
    let grade: mysql::Value = 0.into();

    let mut bg = backend.lock().unwrap();
    for (id, answer) in &data.answers {
        bg.replace(
            "answers",
            (
                apikey.user.clone(),
                num.clone(),
                *id,
                answer.clone(),
                PCon::new(ts.clone(), NoPolicy {}),
                PCon::new(grade.clone(), NoPolicy {}),
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
                    let recipients = if num < 90 {
                        config.staff.clone()
                    } else {
                        config.admins.clone()
                    };

                    email::send(
                        bg.log.clone(),
                        user,
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
