use std::{cmp, env, error, fmt, iter, path, process};

use crate::{metadata, symlink};

pub struct App<T> {
    args: T,
}

impl<T> App<T>
where
    T: Iterator<Item = String>,
{
    pub fn run(self) -> Status {
        execute(self.args).into()
    }
}

impl Default for App<iter::Skip<env::Args>> {
    fn default() -> Self {
        Self {
            args: env::args().skip(1),
        }
    }
}

impl<T> From<T> for App<T>
where
    T: Iterator<Item = String>,
{
    fn from(value: T) -> Self {
        Self { args: value }
    }
}

pub enum Status {
    Ok,
    Err(Error),
}

impl From<Result<(), Error>> for Status {
    fn from(value: Result<(), Error>) -> Self {
        match value {
            Ok(()) => Status::Ok,
            Err(err) => Status::Err(err),
        }
    }
}

impl From<Status> for Result<(), Error> {
    fn from(value: Status) -> Self {
        match value {
            Status::Ok => Self::Ok(()),
            Status::Err(err) => Self::Err(err),
        }
    }
}

impl process::Termination for Status {
    fn report(self) -> process::ExitCode {
        match self {
            Self::Ok => process::ExitCode::SUCCESS,
            Self::Err(err) => {
                eprintln!("Error: {err}");
                process::ExitCode::FAILURE
            }
        }
    }
}

#[derive(Debug)]
pub enum Error {
    ActivateMetadata(metadata::Error),
    ActivateMetadataStoreNotFound(path::PathBuf),
    ActivateNoMetadataActual,
    ActivateNoMetadataStore,
    ActivateNoTargetRoot,
    ActivateSymlinkCreate {
        err: symlink::Error,
        source: path::PathBuf,
        target: path::PathBuf,
    },
    ActivateSymlinkRemove {
        err: symlink::Error,
        source: path::PathBuf,
        target: path::PathBuf,
    },
    CmdMissing,
    CmdUnknown(String),
    RegisterNoMetadata,
    RegisterNoSource,
    RegisterNoTarget,
    RegisterNewEntryCreate(metadata::Error),
    RegisterNewEntryWrite(metadata::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::ActivateMetadata(err) => write!(out, "activate: metadata: {}", err),
            Self::ActivateMetadataStoreNotFound(path) => {
                write!(out, "activate: metadata store path is not found: {}", path.display())
            }
            Self::ActivateNoMetadataActual => write!(out, "activate: actual metadata path not provided"),
            Self::ActivateNoMetadataStore => write!(out, "activate: store metadata path not provided"),
            Self::ActivateNoTargetRoot => write!(out, "activate: target root not provided"),
            Self::ActivateSymlinkCreate { err, source, target } => write!(
                out,
                "activate: create symlink {} -> {}: {}",
                source.display(),
                target.display(),
                err
            ),
            Self::ActivateSymlinkRemove { err, source, target } => write!(
                out,
                "activate: remove symlink {} -> {}: {}",
                source.display(),
                target.display(),
                err
            ),
            Self::CmdMissing => write!(out, "command not provided"),
            Self::RegisterNoMetadata => write!(out, "register: metadata path not provided"),
            Self::RegisterNoSource => write!(out, "register: link source not provided"),
            Self::RegisterNoTarget => write!(out, "register: link target not provided"),
            Self::RegisterNewEntryCreate(err) => write!(out, "register: {}", err),
            Self::RegisterNewEntryWrite(err) => write!(out, "register: {}", err),
            Self::CmdUnknown(value) => write!(out, "unknown command: {value}"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(match self {
            Self::ActivateMetadata(err) => err,
            Self::RegisterNewEntryCreate(err) | Self::RegisterNewEntryWrite(err) => err,
            _ => return None,
        })
    }
}

fn execute(mut args: impl Iterator<Item = String>) -> Result<(), Error> {
    let raw_name = args.next().ok_or(Error::CmdMissing)?;
    match &*raw_name {
        "register" => {
            let metadata_path = path::PathBuf::from(args.next().ok_or(Error::RegisterNoMetadata)?);
            let source = args.next().ok_or(Error::RegisterNoSource)?;
            let target = args.next().ok_or(Error::RegisterNoTarget)?;
            execute_register(metadata_path, source, target)
        }
        "activate" => {
            let metadata_actual_path = path::PathBuf::from(args.next().ok_or(Error::ActivateNoMetadataActual)?);
            let metadata_store_path = path::PathBuf::from(args.next().ok_or(Error::ActivateNoMetadataStore)?);
            let target_root = path::PathBuf::from(args.next().ok_or(Error::ActivateNoTargetRoot)?);
            execute_activate(metadata_actual_path, metadata_store_path, target_root)
        }
        _ => Err(Error::CmdUnknown(raw_name)),
    }
}

fn execute_register(metadata_path: path::PathBuf, source: String, target: String) -> Result<(), Error> {
    let new_entry = metadata::NewEntry::create(source, target).map_err(Error::RegisterNewEntryCreate)?;
    metadata::write_entry(metadata_path, &new_entry).map_err(Error::RegisterNewEntryWrite)?;
    Ok(())
}

fn execute_activate(
    metadata_actual_path: path::PathBuf,
    metadata_store_path: path::PathBuf,
    target_root: path::PathBuf,
) -> Result<(), Error> {
    if !metadata_store_path.exists() {
        return Err(Error::ActivateMetadataStoreNotFound(metadata_store_path));
    }
    let status = ActivationStatus::read(&metadata_store_path, &metadata_actual_path)?;
    if status.should_remove() {
        remove(&metadata_actual_path, &target_root)?;
    }
    if status.should_create() {
        create(metadata_store_path, metadata_actual_path, target_root)?;
    }
    Ok(())
}

enum ActivationStatus {
    Create,
    Overwrite,
    Skip,
}

impl ActivationStatus {
    fn read(
        metadata_store_path: impl AsRef<path::Path>,
        metadata_actual_path: impl AsRef<path::Path>,
    ) -> Result<Self, Error> {
        Ok(if metadata_actual_path.as_ref().exists() {
            let ordering =
                metadata::compare(metadata_store_path, metadata_actual_path).map_err(Error::ActivateMetadata)?;
            if ordering != cmp::Ordering::Equal {
                Self::Overwrite
            } else {
                Self::Skip
            }
        } else {
            Self::Create
        })
    }

    fn should_remove(&self) -> bool {
        match self {
            Self::Create | Self::Skip => false,
            Self::Overwrite => true,
        }
    }

    fn should_create(&self) -> bool {
        match self {
            Self::Create | Self::Overwrite => true,
            Self::Skip => false,
        }
    }
}

fn remove(metadata_path: impl AsRef<path::Path>, target_root: impl AsRef<path::Path>) -> Result<(), Error> {
    let metadata_path = metadata_path.as_ref();
    let entries = metadata::read_entries(metadata_path, target_root).map_err(Error::ActivateMetadata)?;
    for entry in entries {
        println!("Removing symlink: {}", entry);
        symlink::remove(&entry.source_path, &entry.target_path).map_err(|err| Error::ActivateSymlinkRemove {
            err,
            source: entry.source_path,
            target: entry.target_path,
        })?;
    }
    println!("Removing metadata: {}", metadata_path.display());
    metadata::remove(metadata_path).map_err(Error::ActivateMetadata)?;
    Ok(())
}

fn create(
    metadata_store_path: path::PathBuf,
    metadata_actual_path: path::PathBuf,
    target_root: path::PathBuf,
) -> Result<(), Error> {
    println!(
        "Copying metadata: {} -> {}",
        metadata_store_path.display(),
        metadata_actual_path.display()
    );
    metadata::copy(&metadata_store_path, &metadata_actual_path).map_err(Error::ActivateMetadata)?;
    let entries = metadata::read_entries(metadata_store_path, target_root).map_err(Error::ActivateMetadata)?;
    for entry in entries {
        println!("Creating symlink: {}", entry);
        symlink::create(&entry.source_path, &entry.target_path).map_err(|err| Error::ActivateSymlinkCreate {
            err,
            source: entry.source_path,
            target: entry.target_path,
        })?;
    }
    Ok(())
}
