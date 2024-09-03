use std::{
    collections::HashSet,
    error,
    fmt,
    fs::File,
    io::{self, BufRead, BufReader, Lines, Write},
    path::{Path, PathBuf},
};

pub fn write_entry(config_path: PathBuf, new_entry: &NewEntry) -> Result<(), Error> {
    let mut config_file = File::options()
        .create(true)
        .append(true)
        .open(config_path)
        .map_err(Error::OpenConfig)?;

    let new_entry_data = new_entry.serialize();
    config_file
        .write_fmt(format_args!("{new_entry_data}"))
        .map_err(Error::WriteNewEntry)
}

pub fn read_entries(config_path: PathBuf, target_root: PathBuf) -> Result<Vec<Entry>, Error> {
    if !target_root.is_absolute() {
        return Err(Error::TargetRootNotAbsolute(target_root));
    }
    if !target_root.is_dir() {
        return Err(Error::TargetRootNotADirectory(target_root));
    }

    let config_parser = ConfigParser::new(config_path)?;

    let mut result = Vec::new();
    let mut seen_targets: HashSet<String> = HashSet::new();
    let mut errors: Vec<Error> = Vec::new();
    for raw_entry in config_parser {
        let (source, target) = raw_entry?;

        if seen_targets.contains(&target) {
            errors.push(Error::EntryTargetDuplicate { source, target });
            continue;
        }
        seen_targets.insert(target.clone());

        match Entry::create(source, target, &target_root) {
            Ok(entry) => result.push(entry),
            Err(err) => errors.push(err),
        }
    }

    if errors.is_empty() {
        Ok(result)
    } else {
        Err(Error::ParseEntries(errors))
    }
}

#[derive(Debug)]
pub struct NewEntry {
    source: String,
    target: String,
}

impl NewEntry {
    pub fn create(source: impl Into<String>, target: impl Into<String>) -> Result<Self, Error> {
        let source = source.into();
        let source_path = Path::new(&source);
        if !source_path.is_absolute() {
            return Err(Error::NewEntrySourceNotAbsolute(source_path.to_owned()));
        }

        let target = target.into();
        let target_path = Path::new(&target);
        if target_path.is_absolute() {
            return Err(Error::NewEntryTargetIsAbsolute(target_path.to_owned()));
        }

        Ok(Self { source, target })
    }

    fn serialize(&self) -> String {
        format!("{}\n{}\n", &self.source, &self.target)
    }
}

#[derive(Debug)]
pub struct Entry {
    pub source_path: PathBuf,
    pub target_path: PathBuf,
}

impl Entry {
    fn create(source: String, target: String, target_root: &Path) -> Result<Self, Error> {
        let source_path = PathBuf::from(&source);
        if !source_path.exists() {
            return Err(Error::EntrySourceNotExists(source_path));
        }
        let target_path = target_root.join(target);
        if target_path.exists() && !target_path.is_symlink() && target_path.is_file() {
            return Err(Error::EntryTargetExists(target_path));
        }
        Ok(Self {
            source_path,
            target_path,
        })
    }
}

impl fmt::Display for Entry {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        write!(out, "{} -> {}", self.source_path.display(), self.target_path.display())
    }
}

struct ConfigParser {
    lines: Lines<BufReader<File>>,
}

impl ConfigParser {
    fn new(path: PathBuf) -> Result<Self, Error> {
        let file = File::options().read(true).open(path).map_err(Error::OpenConfig)?;
        let reader = BufReader::new(file);
        let lines = reader.lines();
        Ok(Self { lines })
    }
}

impl Iterator for ConfigParser {
    type Item = Result<(String, String), Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.lines.next().map(|source| {
            source.map_err(Error::ParseEntrySource).and_then(|source| {
                self.lines
                    .next()
                    .map(|target| target.map_err(Error::ParseEntryTarget))
                    .transpose()
                    .and_then(|target| target.ok_or(Error::ParseEntryTargetMissing))
                    .map(|target| (source, target))
            })
        })
    }
}

#[derive(Debug)]
pub enum Error {
    EntrySourceNotExists(PathBuf),
    EntryTargetDuplicate { source: String, target: String },
    EntryTargetExists(PathBuf),
    NewEntrySourceNotAbsolute(PathBuf),
    NewEntryTargetIsAbsolute(PathBuf),
    OpenConfig(io::Error),
    ParseEntries(Vec<Error>),
    ParseEntrySource(io::Error),
    ParseEntryTarget(io::Error),
    ParseEntryTargetMissing,
    TargetRootNotAbsolute(PathBuf),
    TargetRootNotADirectory(PathBuf),
    WriteNewEntry(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::EntrySourceNotExists(path) => write!(out, "entry: source not exists: {}", path.display()),
            Self::EntryTargetDuplicate { source, target } => {
                write!(out, "entry: target duplicate: {source} -> {target}",)
            }
            Self::EntryTargetExists(path) => write!(out, "entry: target already exists: {}", path.display()),
            Self::NewEntrySourceNotAbsolute(path) => {
                write!(out, "new entry: source is not an absolute path: {}", path.display())
            }
            Self::NewEntryTargetIsAbsolute(path) => {
                write!(out, "new entry: target must be a relative path: {}", path.display())
            }
            Self::OpenConfig(err) => write!(out, "open config: {err}"),
            Self::ParseEntries(errors) => {
                let msg = errors
                    .iter()
                    .fold(String::from("parse entries:"), |acc, x| format!("{acc}\n\t{x}"));
                write!(out, "{msg}")
            }
            Self::ParseEntrySource(err) => write!(out, "parse entry source: {err}"),
            Self::ParseEntryTarget(err) => write!(out, "parse entry target: {err}"),
            Self::ParseEntryTargetMissing => write!(out, "parse entry target: missing"),
            Self::TargetRootNotAbsolute(path) => write!(out, "target root is not an absolute path: {}", path.display()),
            Self::TargetRootNotADirectory(path) => write!(out, "target root is not a directory: {}", path.display()),
            Self::WriteNewEntry(err) => write!(out, "write new entry: {err}"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(match self {
            Self::EntrySourceNotExists(_)
            | Self::EntryTargetDuplicate { .. }
            | Self::EntryTargetExists(_)
            | Self::NewEntrySourceNotAbsolute(_)
            | Self::NewEntryTargetIsAbsolute(_) => return None,
            Self::OpenConfig(err) => err,
            Self::ParseEntries(_) => return None,
            Self::ParseEntrySource(err) | Self::ParseEntryTarget(err) => err,
            Self::ParseEntryTargetMissing => return None,
            Self::TargetRootNotAbsolute(_) | Self::TargetRootNotADirectory(_) => return None,
            Self::WriteNewEntry(err) => err,
        })
    }
}
