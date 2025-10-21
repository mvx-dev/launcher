use std::{
    ffi::OsStr,
    fs::{self, File},
    io::Read,
};

use freedesktop_file_parser::{DesktopFile, EntryType, parse};

#[derive(Debug, Default)]
pub struct AppEntry<'a> {
    pub name: &'a str,
    pub exec: &'a str,
    pub keywords: Vec<String>,
    pub categories: Vec<String>,
}

#[derive(Debug)]
pub struct LauncherState<'a> {
    pub entries: Vec<AppEntry<'a>>,
    pub query: String,
    pub results: Vec<AppEntry<'a>>,
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

fn parse_desktop_entries(
    desktop_entries: &Vec<DesktopFile>,
) -> Result<Vec<AppEntry>, Box<dyn std::error::Error>> {
    let app_entries: Vec<AppEntry> = desktop_entries
        .iter()
        .filter_map(|entry| {
            if let EntryType::Application(app) = &entry.entry.entry_type {
                let mut new_entry = AppEntry::default();

                new_entry.name = entry.entry.name.default.as_ref();
                new_entry.exec = app.exec.as_ref()?;

                Some(new_entry)
            } else {
                None
            }
        })
        .collect();

    Ok(app_entries)
}
