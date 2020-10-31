#[macro_use]
extern crate lazy_static;

mod config;
mod filesystem;
mod task;

use task::Runnable;


fn main() {
    let (mut params, tasks) = config::parse_config();
    tasks.run(&mut params);
}
