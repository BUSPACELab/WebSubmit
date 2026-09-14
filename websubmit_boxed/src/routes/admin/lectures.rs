use std::sync::{Arc, Mutex};

use rocket::State;
use sesame::context::Context;
use sesame::fold::fold;
use sesame::pcon::PCon;
use sesame::policy::{AnyPolicy, AnyPolicyDyn, NoPolicy};
use sesame::verified::VerifiedRegion;
use sesame_rocket::render::PConRender;
use sesame_rocket::rocket::{get, post, FromPConForm, PConForm, PConRedirect, PConTemplate};

use crate::db::MySqlBackend;
use crate::guards::admin::Admin;
use crate::policies::{ContextData, UserEmailPolicy};
use crate::models::QuestionModel;

#[derive(PConRender)]
struct LectureAddRender {
}

#[derive(PConRender)]
struct LectureAdminRender {
    lec_id: PCon<u8, NoPolicy>,
    title: String,
    presenters: PCon<String, AnyPolicy>,
    questions: Vec<QuestionModel>,
}

#[derive(Debug, FromPConForm)]
pub(crate) struct LectureAddForm {
    lec_id: PCon<u8, NoPolicy>,
    lec_label: PCon<String, NoPolicy>,
}

#[derive(Debug, FromPConForm)]
pub(crate) struct LectureEditForm {
    lec_name: PCon<String, NoPolicy>,
    lec_presenters: PCon<String, NoPolicy>,
}

#[get("/")]
pub(crate) fn lec_add(_adm: Admin, context: Context<ContextData>) -> PConTemplate {
    let ctx = LectureAddRender {};
    PConTemplate::render("admin/lectures/add", &ctx, context).unwrap()
}

#[post("/", data = "<data>")]
pub(crate) fn lec_add_submit(
    _adm: Admin,
    data: PConForm<LectureAddForm>,
    backend: &State<Arc<Mutex<MySqlBackend>>>,
    context: Context<ContextData>,
) -> PConRedirect {
    let data = data.into_inner();

    let lec_id = data.lec_id.into_pcon::<u64, NoPolicy>();

    // insert into MySql if not exists
    let mut bg = backend.lock().unwrap();
    bg.insert("lectures", (lec_id, data.lec_label), context);
    drop(bg);

    PConRedirect::to2("/leclist")
}

#[get("/<num>")]
pub(crate) fn lec(
    _adm: Admin,
    num: PCon<u8, NoPolicy>,
    backend: &State<Arc<Mutex<MySqlBackend>>>,
    context: Context<ContextData>,
) -> PConTemplate {
    let key = num.clone().into_pcon::<u64, NoPolicy>();

    let mut bg = backend.lock().unwrap();
    let mut res = bg.query_questions("lecture_id", (key.clone(),), context.clone());
    let label_res = bg.query_lectures("id", (key.clone(),), context.clone());
    let presenters_res = bg.query_presenters("lecture_id", (key,), context.clone());
    drop(bg);

    // lectures carry no policy; presenter emails do.
    let title: String = match label_res.into_iter().next() {Some(lecture) => lecture.label.discard_box(), None => String::new()};
    let presenters: Vec<PCon<String, UserEmailPolicy>> = presenters_res
        .into_iter()
        .map(|presenter| presenter.email)
        .collect();
    let presenters: PCon<String, AnyPolicy> = fold::<dyn AnyPolicyDyn, _>(presenters)
        .unwrap()
        .into_verified(VerifiedRegion::new(|emails: Vec<String>| emails.join(",")));

    res.sort_by_key(|q| q.question_number.clone().discard_box());
    let questions = res;
    // TODO(babman): sorting.
    // questions.sort_by(|a, b| a.id.cmp(&b.id));

    let ctx = LectureAdminRender {lec_id: num, title, presenters, questions: questions};

    PConTemplate::render("admin/lectures/manage", &ctx, context).unwrap()
}

#[post("/<num>", data = "<data>")]
pub(crate) fn lec_edit_submit(
    _adm: Admin,
    num: PCon<u8, NoPolicy>,
    data: PConForm<LectureEditForm>,
    backend: &State<Arc<Mutex<MySqlBackend>>>,
    context: Context<ContextData>,
) -> PConRedirect {
    let data = data.into_inner();
    let key = num.clone().into_pcon::<u64, NoPolicy>();

    let mut bg = backend.lock().unwrap();
    bg.exec(
        "UPDATE lectures SET label = ? WHERE id = ?",
        (data.lec_name, key.clone()),
        context.clone(),
    );

    // Presenters are given as a comma separated list, and replace the existing
    // ones wholesale.
    bg.exec(
        "DELETE FROM presenters WHERE lecture_id = ?",
        (key.clone(),),
        context.clone(),
    );
    let presenters: Vec<PCon<String, NoPolicy>> = data
        .lec_presenters
        .into_verified(VerifiedRegion::new(|presenters: String| {
            presenters
                .split(',')
                .map(|presenter| presenter.trim().to_string())
                .filter(|presenter| !presenter.is_empty())
                .collect::<Vec<String>>()
        }))
        .fold_in();
    for presenter in presenters {
        bg.insert(
            "presenters(lecture_id, email)",
            (key.clone(), presenter),
            context.clone(),
        );
    }
    drop(bg);

    PConRedirect::to("/admin/lec/{}", (&num,), context).unwrap()
}
