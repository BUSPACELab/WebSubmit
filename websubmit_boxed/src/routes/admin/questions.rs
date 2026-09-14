use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rocket::State;
use sesame::context::Context;
use sesame::pcon::{EitherPCon, PCon};
use sesame::policy::NoPolicy;
use sesame::verified::VerifiedRegion;
use sesame_rocket::rocket::{get, post, FromPConForm, PConForm, PConRedirect, PConTemplate};

use crate::db::MySqlBackend;
use crate::guards::admin::Admin;
use crate::policies::ContextData;

#[derive(Debug, FromPConForm)]
pub(crate) struct QuestionForm {
    q_prompt: PCon<String, NoPolicy>,
}

#[post("/<num>", data = "<data>")]
pub(crate) fn addq(
    _adm: Admin,
    num: PCon<u8, NoPolicy>,
    data: PConForm<QuestionForm>,
    backend: &State<Arc<Mutex<MySqlBackend>>>,
    context: Context<ContextData>,
) -> PConRedirect {
    let data = data.into_inner();

    let key = num.clone().into_pcon::<u64, NoPolicy>();

    let mut bg = backend.lock().unwrap();

    // Find question number within lecture.
    let existing = bg.query_questions("lecture_id", (key.clone(),), context.clone());
    let question_number: u64 = existing.len() as u64 + 1;

    bg.insert(
        "questions(lecture_id, question_number, question)",
        (
            key,
            PCon::new(question_number, NoPolicy {}),
            data.q_prompt,
        ),
        context.clone(),
    );
    drop(bg);

    PConRedirect::to("/admin/lec/{}", (&num,), context).unwrap()
}

#[get("/<num>/<qid>")]
pub(crate) fn editq(
    _adm: Admin,
    num: PCon<u8, NoPolicy>,
    qid: PCon<u64, NoPolicy>,
    backend: &State<Arc<Mutex<MySqlBackend>>>,
    context: Context<ContextData>,
) -> PConTemplate {
    let mut bg = backend.lock().unwrap();
    let res = bg.query_questions(
        "id",
        (qid.clone(),),
        context.clone(),
    );
    drop(bg);

    let mut ctx: HashMap<&str, EitherPCon<String, NoPolicy>> = HashMap::new();
    if let Some(question) = res.into_iter().next() {
        ctx.insert("lec_qprompt", EitherPCon::Right(question.question));
        ctx.insert(
            "lec_qnum",
            EitherPCon::Right(
                question
                    .question_number
                    .into_verified(VerifiedRegion::new(|qnum: u64| format!("{}", qnum))),
            ),
        );
    }
    ctx.insert(
        "id",
        EitherPCon::Right(qid.into_verified(VerifiedRegion::new(|qid| format!("{}", qid)))),
    );
    ctx.insert(
        "lec_id",
        EitherPCon::Right(num.into_verified(VerifiedRegion::new(|num| format!("{}", num)))),
    );
    PConTemplate::render("admin/questions", &ctx, context).unwrap()
}

#[post("/editq/<num>/<qid>", data = "<data>")]
pub(crate) fn editq_submit(
    _adm: Admin,
    num: PCon<u8, NoPolicy>,
    qid: PCon<u64, NoPolicy>,
    data: PConForm<QuestionForm>,
    backend: &State<Arc<Mutex<MySqlBackend>>>,
    context: Context<ContextData>,
) -> PConRedirect {
    let data = data.into_inner();
    let mut bg = backend.lock().unwrap();
    bg.exec(
        "UPDATE questions SET question = ? WHERE id = ?",
        (data.q_prompt, qid),
        context.clone(),
    );
    drop(bg);

    PConRedirect::to("/admin/lec/{}", (&num,), context).unwrap()
}
