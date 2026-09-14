use std::sync::{Arc, Mutex};

use rocket::Build;
use rocket::fs::FileServer;
use rocket_dyn_templates::Template;
use sesame_rocket::rocket::{routes, SesameRocket, SesameRoute};
use slog::o;

mod config;
mod db;
mod email;
mod guards;
mod models;
mod policies;
mod routes;

fn new_logger() -> slog::Logger {
    use slog::Drain;
    use slog::Logger;
    use slog_term::term_full;
    Logger::root(Mutex::new(term_full()).fuse(), o!())
}

pub use config::Config;

pub fn make_rocket(config: Config) -> SesameRocket<Build> {
    let backend = Arc::new(Mutex::new(
        db::MySqlBackend::new(
            &config.db_user,
            &config.db_password,
            &config.db_name,
            Some(new_logger()),
            config.prime,
        )
        .unwrap(),
    ));

    let template_dir = config.template_dir.clone();
    let resource_dir = config.resource_dir.clone();

    // rocket_dyn_templates validates its own configured template directory (default
    // "templates", resolved against the working directory) before the custom callback
    // below runs, so point it at the directory from our config.
    std::env::set_var("ROCKET_TEMPLATE_DIR", &template_dir);
    let template = Template::try_custom(move |engines| {
        let result = engines
            .handlebars
            .register_templates_directory(".hbs", &template_dir);
        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(Box::new(e)),
        }
    });

    SesameRocket::build()
        .attach(template)
        .manage(backend)
        .manage(config)
        .mount(
            "/css",
            SesameRoute::from(FileServer::from(format!("{}/css", resource_dir))),
        )
        .mount(
            "/js",
            SesameRoute::from(FileServer::from(format!("{}/js", resource_dir))),
        )
        .mount("/", routes![routes::index::index])
        .mount("/login", routes![routes::index::login])
        .mount("/apikey/generate", routes![routes::apikey::generate::generate])
        .mount("/apikey/check", routes![routes::apikey::check::check])
        .mount("/leclist", routes![routes::leclist::leclist])
        .mount(
            "/questions",
            routes![
                routes::students::questions::questions,
                routes::students::questions::questions_submit
            ],
        )
        .mount(
            "/answers",
            routes![
                routes::admin::answers::composed_answers,
                routes::answers::presenters::answers_for_presenters
            ],
        )
        .mount(
            "/admin/lec/add",
            routes![
                routes::admin::lectures::lec_add,
                routes::admin::lectures::lec_add_submit
            ],
        )
        .mount("/admin/lec/edit", routes![routes::admin::lectures::lec_edit_submit])
        .mount(
            "/admin/lec",
            routes![
                routes::admin::lectures::lec,
                routes::admin::questions::addq,
                routes::admin::questions::editq,
                routes::admin::questions::editq_submit
            ],
        )
        .mount("/admin/users", routes![routes::admin::users::get_registered_users])
        .mount("/admin/grading", routes![routes::admin::users::grading])
}
