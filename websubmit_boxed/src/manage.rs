use std::sync::{Arc, Mutex};

use mysql::prelude::FromValue;

use rocket::http::Status;
use rocket::outcome::IntoOutcome;
use rocket::State;

use serde::Serialize;

use crate::apikey::ApiKey;
use crate::backend::MySqlBackend;
use crate::config::Config;
use crate::policies::ContextData;

use sesame::pcon::PCon;
use sesame_rocket::render::PConRender;
use sesame::context::Context;
use sesame_mysql::{from_value, PConRow};
use sesame::policy::AnyPolicy;
use sesame::verified::VerifiedRegion;
use sesame_rocket::rocket::{get, PConRequest, PConRequestOutcome, PConTemplate, FromPConRequest};

pub(crate) struct Manager;

#[derive(Debug)]
pub(crate) enum ManagerError {
    Unauthorized,
}

#[rocket::async_trait]
impl<'a, 'r> FromPConRequest<'a, 'r> for Manager {
    type PConError = ManagerError;

    async fn from_pcon_request(
        request: PConRequest<'a, 'r>,
    ) -> PConRequestOutcome<Self, Self::PConError> {
        let apikey = request.guard::<ApiKey>().await.unwrap();
        let cfg = request.guard::<&State<Config>>().await.unwrap();

        let manager = apikey.user.verified(VerifiedRegion::new(|user: &String| {
            if cfg.managers.contains(&user) {
                Some(Manager)
            } else {
                None
            }
        }));

        let manager = match manager.fold_in() {
            None => None,
            Some(_) => Some(Manager),
        };
        manager.into_outcome((Status::Unauthorized, ManagerError::Unauthorized))
    }
}

#[derive(PConRender)]
pub(crate) struct Aggregate<T: Serialize> {
    property: PCon<T, AnyPolicy>,
    average: PCon<f64, AnyPolicy>,
}

#[derive(PConRender)]
struct AggregateGenderContext {
    aggregate: Vec<Aggregate<String>>,
    parent: String,
}

#[derive(PConRender)]
struct AggregateRemoteContext {
    aggregate: Vec<Aggregate<bool>>,
    parent: String,
}

fn transform<T: Serialize + FromValue>(
    agg: Vec<PConRow>,
) -> Vec<Aggregate<T>> {
    agg.into_iter()
        .map(|r| Aggregate {
            property: from_value(r.get(0).unwrap()).unwrap(),
            average: from_value(r.get(1).unwrap()).unwrap(),
        })
        .collect()
}

#[get("/gender")]
pub(crate) fn get_aggregate_gender(
    _manager: Manager,
    backend: &State<Arc<Mutex<MySqlBackend>>>,
    context: Context<ContextData>,
) -> PConTemplate {
    let mut bg = backend.lock().unwrap();
    let grades = bg.prep_exec("SELECT * from agg_gender", (), context.clone());
    drop(bg);

    let ctx = AggregateGenderContext {
        aggregate: transform(grades),
        parent: String::from("layout"),
    };

    PConTemplate::render("manage/aggregate", &ctx, context).unwrap()
}

#[get("/remote_buggy")]
pub(crate) fn get_aggregate_remote_buggy(
    _manager: Manager,
    backend: &State<Arc<Mutex<MySqlBackend>>>,
    context: Context<ContextData>,
) -> PConTemplate {
    let mut bg = backend.lock().unwrap();
    let grades = bg.prep_exec("SELECT * from agg_remote", (), context.clone());
    drop(bg);

    let ctx = AggregateRemoteContext {
        aggregate: transform(grades),
        parent: String::from("layout"),
    };

    PConTemplate::render("manage/aggregate", &ctx, context).unwrap()
}

#[get("/remote")]
pub(crate) fn get_aggregate_remote(
    _manager: Manager,
    backend: &State<Arc<Mutex<MySqlBackend>>>,
    context: Context<ContextData>,
) -> PConTemplate {
    let mut bg = backend.lock().unwrap();
    let grades = bg.prep_exec(
        "SELECT * from agg_remote WHERE ucount >= 10",
        (),
        context.clone(),
    );
    drop(bg);

    let ctx = AggregateRemoteContext {
        aggregate: transform(grades),
        parent: String::from("layout"),
    };

    PConTemplate::render("manage/aggregate", &ctx, context).unwrap()
}

#[derive(PConRender)]
pub(crate) struct InfoForEmployers {
    email: PCon<String, AnyPolicy>,
    average_grade: PCon<f64, AnyPolicy>,
}

#[derive(PConRender)]
struct InfoForEmployersContext {
    users: Vec<InfoForEmployers>,
    parent: String,
}

#[get("/employers")]
pub(crate) fn get_list_for_employers(
    _manager: Manager,
    backend: &State<Arc<Mutex<MySqlBackend>>>,
    _config: &State<Config>,
    context: Context<ContextData>,
) -> PConTemplate {
    let mut bg = backend.lock().unwrap();
    let res = bg.prep_exec(
        "SELECT * from employers_release WHERE consent = 1",
        (),
        context.clone(),
    );
    drop(bg);

    let users = res
        .into_iter()
        .map(|r| InfoForEmployers {
            email: from_value(r.get(0).unwrap()).unwrap(),
            average_grade: from_value(r.get(1).unwrap()).unwrap(),
        })
        .collect();

    let ctx = InfoForEmployersContext {
        users: users,
        parent: "layout".into(),
    };
    PConTemplate::render("manage/users", &ctx, context).unwrap()
}

#[get("/employers_buggy")]
pub(crate) fn get_list_for_employers_buggy(
    _manager: Manager,
    backend: &State<Arc<Mutex<MySqlBackend>>>,
    _config: &State<Config>,
    context: Context<ContextData>,
) -> PConTemplate {
    let mut bg = backend.lock().unwrap();
    let res = bg.prep_exec("SELECT * from employers_release", (), context.clone());
    drop(bg);

    let users = res
        .into_iter()
        .map(|r| InfoForEmployers {
            email: from_value(r.get(0).unwrap()).unwrap(),
            average_grade: from_value(r.get(1).unwrap()).unwrap(),
        })
        .collect();

    let ctx = InfoForEmployersContext {
        users: users,
        parent: "layout".into(),
    };
    PConTemplate::render("manage/users", &ctx, context).unwrap()
}
