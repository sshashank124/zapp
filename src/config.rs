use std::fs;
use std::path::PathBuf;

use serde_yaml::Value;
use tera::{Context, Tera};

use crate::task::Task;
use crate::filesystem;


lazy_static! {
    static ref CONFIG_DIR: PathBuf = {
        let mut conf_dir = dirs::config_dir().unwrap();
        conf_dir.push("zapp");
        conf_dir
    };

    pub static ref TEMPLATES: Tera = {
        let mut templates_dir = CONFIG_DIR.clone();
        templates_dir.push("templates/**/*");
        Tera::new(templates_dir.to_str().unwrap()).unwrap()
    };
}


#[derive(Debug)]
pub struct Params {
    pub context: Context,
    pub depth: usize,
}


impl Params {
    pub fn new(context: Context) -> Self {
        Self { context, depth: 0 }
    }
}


pub fn asset(asset_dir: &str, asset_path: &str) -> PathBuf {
    let asset_path = filesystem::expand_path(asset_path);

    if asset_path.is_absolute() {
        asset_path
    } else {
        let mut path = CONFIG_DIR.clone();
        path.push(asset_dir); path.push(asset_path);
        path
    }
}


pub fn parse_config() -> (Params, Task) {
    let mut conf_file = CONFIG_DIR.clone();
    conf_file.push("config.yaml");
    let file = fs::read_to_string(conf_file)
                  .expect("unable to open config.yaml");

    let config = serde_yaml::from_str::<Value>(&file)
                            .expect("unable to parse config file");

    let params = serde_yaml::from_str::<Value>(&param_strs(&config["params"]))
                                   .expect("unable to parse param files");

    let context = Context::from_serialize(&params)
                         .expect("unable to create params context");

    let task = Task::parse_from_config("main", &config["tasks"]);

    (Params::new(context), task)
}


fn param_strs(config: &Value) -> String {
    config.as_sequence().unwrap().iter()
          .map(Value::as_str).map(Option::unwrap)
          .map(|s| asset("params", s))
          .map(fs::read_to_string)
          .collect::<Result<Vec<_>, _>>()
          .expect("unable to load param files")
          .join("\n")
}
