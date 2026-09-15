use std::sync::{Arc, Mutex};

use handlebars::handlebars_helper;
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

// Render its arguments, concatenated, as one quoted CSV field: embedded quotes
// are doubled per RFC 4180, and a leading character that a spreadsheet would
// read as the start of a formula is prefixed with an apostrophe so that
// user-supplied text (e.g. an email address) cannot become a live formula.
handlebars_helper!(csv: |*args| {
    let mut field = String::new();
    for arg in args {
        match arg {
            serde_json::Value::String(s) => field.push_str(s),
            other => field.push_str(&other.to_string()),
        }
    }
    if field.starts_with(&['=', '+', '-', '@', '\t', '\r'][..]) {
        field.insert(0, '\'');
    }
    format!("\"{}\"", field.replace('"', "\"\""))
});


pub fn make_rocket(config: Config) -> SesameRocket<Build> {
    let backend = Arc::new(Mutex::new(
        db::MySqlBackend::new(
            &config.db_user,
            &config.db_password,
            &config.db_addr,
            &config.db_name,
            Some(new_logger()),
            config.prime,
        )
        .unwrap(),
    ));

    let template_dir = config.template_dir.clone();
    let resource_dir = config.resource_dir.clone();

    // Rocket reads its own listener settings from its figment, which never sees
    // our config file, so hand it the port the same way as the template dir
    // below: through the environment it already reads.
    std::env::set_var("ROCKET_PORT", config.port.to_string());

    // rocket_dyn_templates validates its own configured template directory (default
    // "templates", resolved against the working directory) before the custom callback
    // below runs, so point it at the directory from our config.
    std::env::set_var("ROCKET_TEMPLATE_DIR", &template_dir);
    let template = Template::try_custom(move |engines| {
        engines.handlebars.register_helper("csv", Box::new(csv));
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
