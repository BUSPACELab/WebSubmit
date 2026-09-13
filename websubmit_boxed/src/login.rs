use rocket::State;
use std::collections::HashMap;

use sesame::context::Context;
use sesame_rocket::rocket::{get, PConTemplate};

use crate::config::Config;
use crate::policies::ContextData;

#[get("/")]
pub(crate) fn login(config: &State<Config>, context: Context<ContextData>) -> PConTemplate {
    let mut ctx = HashMap::new();
    ctx.insert("CLASS_ID", config.class.clone());
    ctx.insert("parent", String::from("layout"));
    PConTemplate::render("login", &ctx, context).unwrap()
}
