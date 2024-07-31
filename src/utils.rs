use crate::lang;
use crate::config::runtime_config::Config;

use colored::Colorize;

pub fn set_sigint_handler(config: &Config) {
    let lang = config.lang.clone();

    ctrlc::set_handler(move || {
        println!("{}", lang::get_loc(&lang).exiting.red());
        std::process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");
}

pub fn get_fancy_progress_bar(size: u64, message: &'static str) -> indicatif::ProgressBar {
    let bar = indicatif::ProgressBar::new(size);
    bar.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("{msg} {bar:40.cyan/blue} {pos}/{len} {percent}%")
            .unwrap()
            .progress_chars("=> "),
    );
    bar.set_message(message);
    bar
}

pub fn print_error_and_exit(message: &str) -> ! {
    println!("{}", message.red());
    std::process::exit(1);
}
