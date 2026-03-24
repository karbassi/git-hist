use clap::{Parser, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = env!("CARGO_PKG_NAME"),
    version = env!("CARGO_PKG_VERSION"),
    about = env!("CARGO_PKG_DESCRIPTION"),
)]
pub struct Args {
    /// Set a target file path
    pub file_path: String,

    /// Show full commit hashes instead of abbreviated commit hashes
    #[arg(long = "full-hash")]
    pub should_use_full_commit_hash: bool,

    /// Set whether the view will scroll beyond the last line
    #[arg(long)]
    pub beyond_last_line: bool,

    /// Set whether the view will emphasize different parts
    #[arg(long = "emphasize-diff")]
    pub should_emphasize_diff: bool,

    /// Use whether authors or committers for names
    #[arg(long = "name-of", value_name = "user", default_value = "author")]
    pub user_for_name: UserType,

    /// Use whether authors or committers for dates
    #[arg(long = "date-of", value_name = "user", default_value = "author")]
    pub user_for_date: UserType,

    /// Set date format: ref. https://docs.rs/chrono/0.4.19/chrono/format/strftime/index.html
    #[arg(long, value_name = "format", default_value = "[%Y-%m-%d]")]
    pub date_format: String,

    /// Set the number of spaces for a tab character (\t)
    #[arg(long = "tab-size", value_name = "size", default_value = "4")]
    tab_size: usize,

    #[arg(skip = String::from("    "))]
    pub tab_spaces: String,
}

impl Args {
    pub fn load() -> Args {
        let mut args = Args::parse();
        args.tab_spaces = " ".repeat(args.tab_size);
        args
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum UserType {
    Author,
    Committer,
}
