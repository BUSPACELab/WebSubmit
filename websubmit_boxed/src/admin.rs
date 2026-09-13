use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rocket::http::Status;
use rocket::outcome::IntoOutcome;
use rocket::State;

use sesame::pcon::{PCon, EitherPCon};
use sesame_rocket::render::PConRender;
use sesame::context::Context;
use sesame_mysql::from_value;
use sesame::policy::{AnyPolicy, AnyPolicyDyn, NoPolicy};
use sesame::verified::{execute_verified, VerifiedRegion};
use sesame_rocket::rocket::{
    get, post, PConForm, PConRedirect, PConRequest, PConRequestOutcome, PConTemplate, FromPConForm,
    FromPConRequest,
};

use crate::apikey::ApiKey;
use crate::backend::MySqlBackend;
use crate::config::Config;
use crate::policies::ContextData;
use crate::questions::{LectureQuestion, LectureQuestionsContext};

pub(crate) struct Admin;

#[derive(Debug)]
pub(crate) enum AdminError {
    Unauthorized,
}

#[rocket::async_trait]
impl<'a, 'r> FromPConRequest<'a, 'r> for Admin {
    type PConError = AdminError;

    async fn from_pcon_request(
        request: PConRequest<'a, 'r>,
    ) -> PConRequestOutcome<Self, Self::PConError> {
        let apikey = request.guard::<ApiKey>().await.unwrap();
        let cfg = request.guard::<&State<Config>>().await.unwrap();

        let admin = apikey.user.verified(VerifiedRegion::new(|user: &String| {
            if cfg.admins.contains(&user) {
                Some(Admin)
            } else {
                None
            }
        }));

        let admin = match admin.fold_in() {
            None => None,
            Some(_) => Some(Admin),
        };
        admin.into_outcome((Status::Unauthorized, AdminError::Unauthorized))
    }
}

#[derive(PConRender)]
struct LecAddContext {
    parent: String,
}

#[get("/")]
pub(crate) fn lec_add(context: Context<ContextData>) -> PConTemplate {
    let ctx = LecAddContext {
        parent: "layout".into(),
    };
    PConTemplate::render("admin/lecadd", &ctx, context).unwrap()
}

#[derive(Debug, FromPConForm)]
pub(crate) struct AdminLecAdd {
    lec_id: PCon<u8, NoPolicy>,
    lec_label: PCon<String, NoPolicy>,
}

#[post("/", data = "<data>")]
pub(crate) fn lec_add_submit(
    _adm: Admin,
    data: PConForm<AdminLecAdd>,
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
    let res = bg.prep_exec(
        "SELECT * FROM questions WHERE lec = ? ORDER BY q",
        (key,),
        context.clone(),
    );
    drop(bg);

    let questions: Vec<LectureQuestion> = res
        .into_iter()
        .map(|r| {
            let id = from_value(r.get(1).unwrap()).unwrap();
            LectureQuestion {
                id: id,
                prompt: from_value(r.get(2).unwrap()).unwrap(),
                answer: PCon::new(None, NoPolicy {}), //TODO(corinn) check fix - this was previously PCon::new(None, vec![])
            }
        })
        .collect();
    // TODO(babman): sorting.
    // questions.sort_by(|a, b| a.id.cmp(&b.id));

    let ctx = LectureQuestionsContext {
        lec_id: num,
        questions: questions,
        parent: "layout".into(),
    };

    PConTemplate::render("admin/lec", &ctx, context).unwrap()
}

#[derive(Debug, FromPConForm)]
pub(crate) struct AddLectureQuestionForm {
    q_id: PCon<u64, NoPolicy>,
    q_prompt: PCon<String, NoPolicy>,
}

#[post("/<num>", data = "<data>")]
pub(crate) fn addq(
    _adm: Admin,
    num: PCon<u8, NoPolicy>,
    data: PConForm<AddLectureQuestionForm>,
    backend: &State<Arc<Mutex<MySqlBackend>>>,
    context: Context<ContextData>,
) -> PConRedirect {
    let data = data.into_inner();

    let mut bg = backend.lock().unwrap();
    bg.insert(
        "questions",
        (
            num.clone().into_pcon::<u64, NoPolicy>(),
            data.q_id.into_pcon::<u64, NoPolicy>(),
            data.q_prompt,
        ),
        context.clone(),
    );
    drop(bg);

    PConRedirect::to("/admin/lec/{}", (&num,), context).unwrap()
}

#[get("/<num>/<qnum>")]
pub(crate) fn editq(
    _adm: Admin,
    num: PCon<u8, NoPolicy>,
    qnum: PCon<u8, NoPolicy>,
    backend: &State<Arc<Mutex<MySqlBackend>>>,
    context: Context<ContextData>,
) -> PConTemplate {
    let mut bg = backend.lock().unwrap();
    let res = bg.prep_exec(
        "SELECT * FROM questions WHERE lec = ?",
        (num.clone().into_pcon::<u64, NoPolicy>(),),
        context.clone(),
    );
    drop(bg);

    let mut ctx: HashMap<&str, EitherPCon<String, NoPolicy>> = HashMap::new();
    for r in res {
        let q = from_value::<u8, AnyPolicy>(r.get(1).unwrap()).unwrap();

        let q_matches = execute_verified::<dyn AnyPolicyDyn, _, _, _>(
            (q, qnum.clone()),
            VerifiedRegion::new(|(q, qnum)| {
                if q == qnum {
                    Some(())
                } else {
                    None
                }
            }),
        )
        .unwrap();

        if q_matches.fold_in().is_some() {
            ctx.insert(
                "lec_qprompt",
                EitherPCon::Right(from_value(r.get(2).unwrap()).unwrap()),
            );
        }
    }
    ctx.insert(
        "lec_id",
        EitherPCon::Right(num.into_verified(VerifiedRegion::new(|num| format!("{}", num)))),
    );
    ctx.insert(
        "lec_qnum",
        EitherPCon::Right(qnum.into_verified(VerifiedRegion::new(|qnum| format!("{}", qnum)))),
    );
    ctx.insert("parent", EitherPCon::Left(String::from("layout")));
    PConTemplate::render("admin/lecedit", &ctx, context).unwrap()
}

#[post("/editq/<num>", data = "<data>")]
pub(crate) fn editq_submit(
    _adm: Admin,
    num: PCon<u8, NoPolicy>,
    data: PConForm<AddLectureQuestionForm>,
    backend: &State<Arc<Mutex<MySqlBackend>>>,
    context: Context<ContextData>,
) -> PConRedirect {
    let data = data.into_inner();
    let mut bg = backend.lock().unwrap();
    bg.prep_exec(
        "UPDATE questions SET question = ? WHERE lec = ? AND q = ?",
        (
            data.q_prompt,
            num.clone().into_pcon::<u64, NoPolicy>(),
            data.q_id,
        ),
        context.clone(),
    );
    drop(bg);

    PConRedirect::to("/admin/lec/{}", (&num,), context).unwrap()
}

#[derive(PConRender, Clone)]
pub(crate) struct User {
    email: PCon<String, NoPolicy>,
    apikey: PCon<String, NoPolicy>,
    is_admin: PCon<bool, NoPolicy>,
}

#[derive(PConRender)]
struct UserContext {
    users: Vec<User>,
    parent: String,
}

#[get("/")]
pub(crate) fn get_registered_users(
    _adm: Admin,
    backend: &State<Arc<Mutex<MySqlBackend>>>,
    config: &State<Config>,
    context: Context<ContextData>,
) -> PConTemplate {
    let mut bg = backend.lock().unwrap();
    let res = bg.prep_exec(
        "SELECT email, is_admin, apikey FROM users",
        (),
        context.clone(),
    );
    drop(bg);

    let users = res
        .into_iter()
        .map(|r| User {
            email: from_value(r.get(0).unwrap()).unwrap(),
            apikey: from_value(r.get(2).unwrap()).unwrap(),
            is_admin: from_value(r.get(0).unwrap())
                .unwrap()
                .into_verified(VerifiedRegion::new(|id| config.admins.contains(&id))),
        })
        .collect();

    let ctx = UserContext {
        users: users,
        parent: "layout".into(),
    };
    PConTemplate::render("admin/users", &ctx, context).unwrap()
}
