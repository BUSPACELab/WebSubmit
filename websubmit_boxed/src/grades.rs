use std::sync::{Arc, Mutex};

use sesame::fold::fold;
use chrono::naive::NaiveDateTime;
use rocket::State;

use sesame::pcon::PCon;
use sesame_rocket::render::PConRender;
use sesame::context::Context;
use sesame_mysql::from_value;
use sesame::policy::{AnyPolicyDyn, NoPolicy};
use sesame::verified::VerifiedRegion;
use sesame_rocket::rocket::{get, post, PConForm, PConRedirect, PConTemplate, FromPConForm};

use crate::admin::Admin;
use crate::backend::MySqlBackend;
use crate::policies::{AnswerAccessPolicy, ContextData};
use crate::questions::LectureAnswer;
use crate::questions::LectureAnswersContext;

#[get("/<num>")]
pub(crate) fn grades(
    _adm: Admin,
    num: PCon<u8, NoPolicy>,
    backend: &State<Arc<Mutex<MySqlBackend>>>,
    context: Context<ContextData>,
) -> PConTemplate {
    let key = num.clone().into_pcon::<u64, NoPolicy>();

    let mut bg = backend.lock().unwrap();
    let res = bg.prep_exec(
        "SELECT * FROM answers WHERE lec = ?",
        (key,),
        context.clone(),
    );
    drop(bg);

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

    let outer_box_answers: PCon<Vec<crate::questions::LectureAnswerOut>, AnswerAccessPolicy> = fold::<dyn AnyPolicyDyn, _>(answers)
        .unwrap()
        .specialize_policy::<AnswerAccessPolicy>()
        .unwrap();

    let ctx = LectureAnswersContext {
        lec_id: num,
        answers: outer_box_answers,
        parent: "layout".into(),
    };

    PConTemplate::render("grades", &ctx, context).unwrap()
}

#[derive(PConRender)]
struct GradeEditContext {
    answer: PCon<String, AnswerAccessPolicy>,
    grade: PCon<u64, AnswerAccessPolicy>,
    lec_id: PCon<u8, NoPolicy>,
    lec_qnum: PCon<u8, NoPolicy>,
    parent: String,
    user: PCon<String, NoPolicy>,
}

#[get("/<user>/<num>/<qnum>")]
pub(crate) fn editg(
    _adm: Admin,
    user: PCon<String, NoPolicy>,
    num: PCon<u8, NoPolicy>,
    qnum: PCon<u8, NoPolicy>,
    backend: &State<Arc<Mutex<MySqlBackend>>>,
    context: Context<ContextData>,
) -> PConTemplate {
    let mut bg = backend.lock().unwrap();
    let res = bg.prep_exec(
        "SELECT answer, grade FROM answers WHERE email = ? AND lec = ? AND q = ?",
        (
            user.clone(),
            num.clone().into_pcon::<u64, NoPolicy>(),
            qnum.clone().into_pcon::<u64, NoPolicy>(),
        ),
        context.clone(),
    );
    drop(bg);

    let r = &res[0];
    let ctx = GradeEditContext {
        answer: from_value(r.get(0).unwrap()).unwrap(),
        grade: from_value(r.get(1).unwrap()).unwrap(),
        user: user,
        lec_id: num,
        lec_qnum: qnum,
        parent: "layout".into(),
    };

    PConTemplate::render("gradeedit", &ctx, context).unwrap()
}

#[derive(Debug, FromPConForm)]
pub(crate) struct EditGradeForm {
    grade: PCon<u64, NoPolicy>,
}

#[post("/editg/<user>/<num>/<qnum>", data = "<data>")]
pub(crate) fn editg_submit(
    _adm: Admin,
    user: PCon<String, NoPolicy>,
    num: PCon<u8, NoPolicy>,
    qnum: PCon<u8, NoPolicy>,
    data: PConForm<EditGradeForm>,
    backend: &State<Arc<Mutex<MySqlBackend>>>,
    context: Context<ContextData>,
) -> PConRedirect {
    let mut bg = backend.lock().unwrap();

    bg.prep_exec(
        "UPDATE answers SET grade = ? WHERE email = ? AND lec = ? AND q = ?",
        (data.grade.clone(), user, num.clone(), qnum),
        context.clone(),
    );
    drop(bg);

    PConRedirect::to("/grades/{}", (&num,), context).unwrap()
}
