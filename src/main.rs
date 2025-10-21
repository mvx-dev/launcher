use std::env;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use freedesktop_file_parser::{DesktopFile, EntryType, parse};
use serde::Deserialize;

use crate::entry_handler::DesktopEntries;

mod entry_handler;

#[derive(Debug, Deserialize, Default)]
struct Config {
    directories: Vec<String>,
}

impl Config {
    fn new() -> Self {
        Config {
            directories: Vec::from(["/usr/share/applications".into()]),
        }
    }
}

fn get_desktop_entries(directory: &str) -> Result<Vec<DesktopFile>, Box<dyn std::error::Error>> {
    let mut desktop_entries: Vec<DesktopFile> = Vec::new();
    let files = fs::read_dir(directory).unwrap();

    for file in files {
        let path = file.unwrap().path();
        let extension = path.extension();

        if extension == Some(OsStr::new("desktop")) {
            let mut file_buffer = File::open(path)?;
            let mut contents = String::new();
            file_buffer.read_to_string(&mut contents)?;

            let desktop_file = parse(&contents).unwrap();
            desktop_entries.push(desktop_file);
        }
    }

    Ok(desktop_entries)
}

// Attempts each step, if one of them fails fallback to the next
// 1) Load the config file specified
// 2) Load the config file from the directory specified
// 3) Load the config file from the default location
// 4) Load the default config using Config::default()
fn load_config(path_string: Option<String>) -> std::io::Result<Config> {
    let mut config = Config::new();

    let default_path = if let Ok(xdg_config) = env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg_config).join(Path::new("launcher"))
    } else if let Ok(home) = env::var("HOME") {
        PathBuf::from(home).join(".config").join("launcher")
    } else {
        panic!("Neither XDG_CONFIG_HOME nor HOME is set");
    };

    let mut path = match path_string {
        Some(path) => {
            println!("Path loaded: {}", path);
            PathBuf::from(path.as_str())
        }
        None => default_path.clone(),
    };

    if path.exists() {
        let metadata = path.metadata()?;

        if metadata.is_file() {
            let content = fs::read_to_string(path)?;
            config = toml::from_str(content.as_str()).unwrap();
            return Ok(config);
        } else if metadata.is_dir() {
            path = path.join("config.toml");
        }
    } else {
        path = default_path.clone();
        path = path.join("config.toml");
    }

    if !path.exists() {
        return Ok(config);
    }

    let content = fs::read_to_string(path)?;
    config = toml::from_str(content.as_str()).unwrap();
    Ok(config)
}

fn show_desktop_file(desktop_file: &DesktopFile) {
    println!("Name: {}", desktop_file.entry.name.default);
    println!("  Type: {}", desktop_file.entry.entry_type);

    if let EntryType::Application(app) = &desktop_file.entry.entry_type {
        if let Some(exec) = app.exec.as_ref() {
            println!("  Exec: {}", exec);
        }
        if let Some(path) = app.path.as_ref() {
            println!("  Path: {}", path);
        }
        if let Some(keywords) = app.keywords.as_ref() {
            println!("  Keywords: {:?}", keywords.default);
        }

        if let Some(categories) = app.categories.as_ref()
            && !categories.is_empty()
        {
            println!("  -- Categories --");
            for category in categories {
                println!("    {}", category);
            }
        }
    }

    let actions = &desktop_file.actions;
    if !actions.is_empty() {
        println!("  -- Actions --");
        for action in actions.values() {
            println!("    Action: {}", action.name.default);
            if let Some(exec) = &action.exec {
                println!("    Action command: {}", exec);
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config_path = std::env::args().nth(1);
    let config = load_config(config_path)?;

    for directory in &config.directories {
        println!("{}", directory);

        let desktop_entries = get_desktop_entries("/home/cheshire/.local/share/applications/")?;

        for desktop_file in &desktop_entries {
            let display = desktop_file.entry.no_display.unwrap_or(true);
            if !display {
                continue;
            }

            show_desktop_file(desktop_file);
            println!();
        }
    }

    Ok(())
}
