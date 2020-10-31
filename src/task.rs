use std::fmt;
use std::fs::{self, File};
use std::os::unix::fs as unixfs;
use std::process::Command;

use serde::Deserialize;
use serde_yaml::Value;

use crate::config::{self, Params};
use crate::filesystem;


pub trait Runnable {
    fn run(&self, params: &mut Params) -> Status;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    Success,
    Failure,
    Skipped,
}


#[derive(Debug, Deserialize)]
pub struct Task {
    #[serde(default)] name: String,
    #[serde(rename="su", default)] as_superuser: bool,
    #[serde(flatten)] variant: TaskType,
}

#[derive(Debug, Deserialize)]
enum TaskType {
    Unknown,
    Group(Vec<Task>),
    #[serde(rename="copy")] Copy(CopyTask),
    #[serde(rename="symlink")] Symlink(SymlinkTask),
    #[serde(rename="template")] Template(TemplateTask),
    #[serde(rename="shell")] Shell(ShellTask),
}

#[derive(Debug, Deserialize)]
struct CopyTask {
    src: String,
    dst: String,
    #[serde(default)]
    #[serde(deserialize_with="filesystem::parse_permissions")]
    mode: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct SymlinkTask {
    src: String,
    dst: String,
}

#[derive(Debug, Deserialize)]
struct TemplateTask {
    src: String,
    dst: String,
    #[serde(default)]
    #[serde(deserialize_with="filesystem::parse_permissions")]
    mode: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct ShellTask(String);


impl Task {
    fn new(name: &str, variant: TaskType) -> Self {
        Self { name: name.to_owned(), as_superuser: false, variant }
    }

    fn group(name: &str, tasks: Vec<Task>) -> Self {
        Self::new(name, TaskType::Group(tasks))
    }

    pub fn parse_from_config(task_name: &str, config: &Value) -> Self {
        let tasks = config.as_sequence().unwrap().iter().map(|t| match t {
            Value::String(s) => Self::load_from_file(s),
            Value::Mapping(m) => {
                let (k, v) = m.iter().next().unwrap();
                Self::parse_from_config(k.as_str().unwrap(), v)
            },
            _ => Self::new("unknown", TaskType::Unknown),
        }).collect();

        Self::group(task_name, tasks)
    }

    fn load_from_file(task_name: &str) -> Self {
        let task_file = File::open(config::asset("tasks", &format!("{}.yaml",
                                                                   task_name)))
                             .expect("unable to open task file");
        let tasks = serde_yaml::from_reader(task_file)
                               .expect("unable to parse task file");

        Self::group(task_name, tasks)
    }
}


impl Runnable for Task {
    fn run(&self, params: &mut Params) -> Status {
        // TODO: handle as_superuser == true
        let status = if !self.as_superuser {
            self.variant.run(params)
        } else { Status::Skipped };
        println!("{: <1$}{name}: {status}", "", params.depth * 2,
                 name=self.name, status=status);
        status
    }
}


impl Runnable for TaskType {
    fn run(&self, params: &mut Params) -> Status {
        match self {
            Self::Unknown => Status::Skipped,
            Self::Group(tasks) => {
                params.depth += 1;
                let status = tasks.iter().map(|t| t.run(params))
                                  .find(|&s| s == Status::Failure)
                                  .unwrap_or(Status::Success);
                params.depth -= 1;
                status
            }
            Self::Copy(task) => task.run(params),
            Self::Symlink(task) => task.run(params),
            Self::Template(task) => task.run(params),
            Self::Shell(task) => task.run(params),
        }
    }
}


impl Runnable for CopyTask {
    fn run(&self, _: &mut Params) -> Status {
        let src = config::asset("files", &self.src);
        let dst = filesystem::expand_path(&self.dst);
        filesystem::create_valid_parent(&dst);

        match fs::copy(src, &dst) {
            Err(_) => return Status::Failure,
            _ => (),
        }

        match filesystem::set_permissions(dst, self.mode) {
            Ok(_) => Status::Success,
            _ => Status::Failure,
        }
    }
}


impl Runnable for SymlinkTask {
    fn run(&self, _: &mut Params) -> Status {
        let src = config::asset("files", &self.src);
        let dst = filesystem::expand_path(&self.dst);
        filesystem::create_valid_parent(&dst);

        match unixfs::symlink(src, dst) {
            Ok(_) => Status::Success,
            _ => Status::Failure,
        }
    }
}


impl Runnable for TemplateTask {
    fn run(&self, params: &mut Params) -> Status {
        let text = match config::TEMPLATES.render(&self.src, &params.context) {
            Ok(s) => s,
            _ => return Status::Failure,
        };
        let dst = filesystem::expand_path(&self.dst);
        filesystem::create_valid_parent(&dst);

        match fs::write(&dst, text) {
            Err(_) => return Status::Failure,
            _ => (),
        }

        match filesystem::set_permissions(dst, self.mode) {
            Ok(_) => Status::Success,
            _ => Status::Failure,
        }
    }
}


impl Runnable for ShellTask {
    fn run(&self, _params: &mut Params) -> Status {
        let exit_code = Command::new("/usr/bin/sh")
                                .args(&["-c", &self.0])
                                .status()
                                .expect("failed to run shell command");
        if exit_code.success() { Status::Success } else { Status::Failure }
    }
}


impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", match self {
            Self::Success => "SUCCESS",
            Self::Failure => "FAILURE",
            Self::Skipped => "SKIPPED",
        })
    }
}
