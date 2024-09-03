use std::{error, fmt, path::PathBuf};

use crate::{command, metadata, symlink};

pub fn link(args: command::ArgsLink) -> Result<(), Error> {
    let entries = metadata::read_entries(args.metadata_path, args.target_root).map_err(Error::LinkReadMetadata)?;
    for entry in entries {
        println!("Creating symlink: {}", entry);
        symlink::create(&entry.source_path, &entry.target_path).map_err(|err| Error::LinkCreate {
            err,
            source: entry.source_path,
            target: entry.target_path,
        })?;
    }
    Ok(())
}

pub fn register(args: command::ArgsRegister) -> Result<(), Error> {
    let new_entry = metadata::NewEntry::create(args.source, args.target).map_err(Error::RegisterNewEntryCreate)?;
    metadata::write_entry(args.metadata_path, &new_entry).map_err(Error::RegisterNewEntryWrite)?;
    Ok(())
}

pub fn unlink(args: command::ArgsUnlink) -> Result<(), Error> {
    let entries = metadata::read_entries(args.metadata_path, args.target_root).map_err(Error::LinkReadMetadata)?;
    for entry in entries {
        println!("Removing symlink: {}", entry);
        symlink::remove(&entry.source_path, &entry.target_path).map_err(|err| Error::LinkRemove {
            err,
            source: entry.source_path,
            target: entry.target_path,
        })?;
    }
    Ok(())
}

#[derive(Debug)]
pub enum Error {
    LinkCreate {
        err: symlink::Error,
        source: PathBuf,
        target: PathBuf,
    },
    LinkReadMetadata(metadata::Error),
    LinkRemove {
        err: symlink::Error,
        source: PathBuf,
        target: PathBuf,
    },
    RegisterNewEntryCreate(metadata::Error),
    RegisterNewEntryWrite(metadata::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::LinkCreate { source, target, err } => write!(
                out,
                "link: create {} -> {}: {}",
                source.display(),
                target.display(),
                err
            ),
            Self::LinkReadMetadata(err) => write!(out, "link: read metadata: {err}"),
            Self::LinkRemove { source, target, err } => write!(
                out,
                "link: remove {} -> {}: {}",
                source.display(),
                target.display(),
                err
            ),
            Self::RegisterNewEntryCreate(err) => write!(out, "register: create new entry: {err}"),
            Self::RegisterNewEntryWrite(err) => write!(out, "register: write new entry: {err}"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(match self {
            Self::LinkCreate { err, .. } => err,
            Self::LinkReadMetadata(err) => err,
            Self::LinkRemove { err, .. } => err,
            Self::RegisterNewEntryCreate(err) => err,
            Self::RegisterNewEntryWrite(err) => err,
        })
    }
}
