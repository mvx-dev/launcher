use std::{
    borrow::Cow,
    ffi::OsStr,
    fs::{self, File},
    io::Read,
};

use freedesktop_file_parser::{DesktopFile, EntryType, parse};
use nucleo_matcher::{
    Matcher, Utf32Str,
    pattern::{CaseMatching, Normalization, Pattern},
};

#[derive(Debug)]
pub struct AppEntry<'a> {
    pub name: Cow<'a, str>,
    pub exec: Cow<'a, str>,
    pub keywords: Vec<Cow<'a, str>>,
    pub categories: Vec<Cow<'a, str>>,

    // Cached UTF-32 data (for fuzzy finding)
    name_buffer: Vec<char>,
    keywords_buffers: Vec<Vec<char>>,
    categories_buffers: Vec<Vec<char>>,

    pub score: Option<i64>,
}

#[derive(Debug)]
pub struct LauncherState<'a> {
    pub entries: Vec<AppEntry<'a>>,
    pub query: String,
    pub results: Vec<AppEntry<'a>>,
}

impl<'a> Default for AppEntry<'_> {
    fn default() -> Self {
        Self {
            name: Cow::Borrowed(""),
            exec: Cow::Borrowed(""),
            keywords: Vec::new(),
            categories: Vec::new(),

            name_buffer: Vec::new(),
            keywords_buffers: Vec::new(),
            categories_buffers: Vec::new(),

            score: None,
        }
    }
}

impl<'a> AppEntry<'_> {
    pub fn new<N, E, K, C>(name: N, exec: E, keywords: Vec<K>, categories: Vec<C>) -> AppEntry<'a>
    where
        N: Into<Cow<'a, str>>,
        E: Into<Cow<'a, str>>,
        K: Into<Cow<'a, str>>,
        C: Into<Cow<'a, str>>,
    {
        let name: Cow<'a, str> = name.into();
        let exec: Cow<'a, str> = exec.into();
        let keywords: Vec<Cow<'a, str>> = keywords.into_iter().map(|k| k.into()).collect();
        let categories: Vec<Cow<'a, str>> = categories.into_iter().map(|c| c.into()).collect();

        let name_buffer = name.chars().collect();
        let keywords_buffers = keywords.iter().map(|k| k.chars().collect()).collect();
        let categories_buffers = categories.iter().map(|c| c.chars().collect()).collect();

        AppEntry {
            name: name,
            exec: exec,
            keywords: keywords,
            categories: categories,

            name_buffer: name_buffer,
            keywords_buffers: keywords_buffers,
            categories_buffers: categories_buffers,

            score: None,
        }
    }

    pub fn name_utf32(&mut self) -> Utf32Str<'_> {
        Utf32Str::new(&self.name, &mut self.name_buffer)
    }

    pub fn keywords_utf32(&mut self) -> impl Iterator<Item = Utf32Str<'_>> {
        self.keywords
            .iter()
            .zip(self.keywords_buffers.iter_mut())
            .map(|(k, buffer)| Utf32Str::new(k, buffer))
            .collect::<Vec<_>>()
            .into_iter()
    }

    pub fn categories_utf32(&mut self) -> impl Iterator<Item = Utf32Str<'_>> {
        self.categories
            .iter()
            .zip(self.categories_buffers.iter_mut())
            .map(|(c, buffer)| Utf32Str::new(c, buffer))
            .collect::<Vec<_>>()
            .into_iter()
    }

    pub fn compute_score(&mut self, matcher: &mut Matcher, pattern: &Pattern) {
        // TODO add caching
        let mut total_score = 0f64;

        if let Some(score) = pattern.score(self.name_utf32(), matcher) {
            total_score += score as f64 * 5f64;
        }

        for keyword in self.keywords_utf32() {
            if let Some(score) = pattern.score(keyword, matcher) {
                total_score += score as f64;
            }
        }

        self.score = Some(total_score as i64);
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

fn parse_desktop_entries(
    desktop_entries: &Vec<DesktopFile>,
) -> Result<Vec<AppEntry>, Box<dyn std::error::Error>> {
    let app_entries: Vec<AppEntry> = desktop_entries
        .iter()
        .filter_map(|entry| {
            if let EntryType::Application(app) = &entry.entry.entry_type {
                let mut new_entry = AppEntry::default();

                new_entry.name = <String as AsRef<str>>::as_ref(&entry.entry.name.default).into();
                new_entry.exec = app.exec.as_ref()?.into();

                Some(new_entry)
            } else {
                None
            }
        })
        .collect();

    Ok(app_entries)
}
