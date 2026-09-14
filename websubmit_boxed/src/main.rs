use clap::{App, Arg};
use websubmit_boxed::{Config, make_rocket};

#[cfg_attr(rustfmt, rustfmt_skip)]
const WEBSUBMIT_USAGE: &'static str = "\
EXAMPLES:
  websubmit
  websubmit -c csci2390-f19.toml";

#[derive(Clone, Debug)]
pub struct Args {
    pub config: Config,
}

pub fn parse_args() -> Args {
    let args = App::new("websubmit")
        .version("0.0.1")
        .about("Class submission system.")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .takes_value(true)
                .value_name("CONFIG_FILE")
                .default_value("sample-config.toml")
                .help("Path to the configuration file for the deployment."),
        )
        .after_help(WEBSUBMIT_USAGE)
        .get_matches();

    Args {
        config: Config::parse(args.value_of("config").expect("Failed to parse config!"))
            .expect("failed to parse config"),
    }
}

#[rocket::main]
async fn main() {
    let args = parse_args();
    let rocket = make_rocket(args.config);
    if let Err(e) = rocket.launch().await {println!("Whoops, didn't launch!");
        drop(e);};
}
