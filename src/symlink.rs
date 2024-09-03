use std::{
    error,
    fmt,
    fs::{canonicalize, create_dir, create_dir_all, remove_file, DirEntry},
    io,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
};

pub fn create(source: impl AsRef<Path>, target: impl AsRef<Path>) -> Result<(), Error> {
    let source = source.as_ref();
    let target = target.as_ref();
    let state = State::new(source, target)?;
    match state {
        State::Equals => Ok(()),
        State::VacantFile {
            source_path,
            target_path,
            target_exists,
        } => {
            if target_exists {
                remove_symlink(target_path)?;
            }
            create_file(source_path, target_path)
        }
        State::VacantDirectory {
            source_path,
            target_path,
        } => create_directory(source_path, target_path),
    }
}

pub fn remove(source: impl AsRef<Path>, target: impl AsRef<Path>) -> Result<(), Error> {
    let source = source.as_ref();
    let target = target.as_ref();
    let state = State::new(source, target)?;
    match state {
        State::Equals => {
            remove_symlink(target)?;
        }
        State::VacantFile { target_exists, .. } => {
            if target_exists {
                remove_symlink(target)?;
            }
        }
        State::VacantDirectory { .. } => {
            remove_directory_entries(source, target)?;
        }
    }
    Ok(())
}

enum State<'a> {
    Equals,
    VacantFile {
        source_path: &'a Path,
        target_path: &'a Path,
        target_exists: bool,
    },
    VacantDirectory {
        source_path: &'a Path,
        target_path: &'a Path,
    },
}

impl<'a> State<'a> {
    fn new(source_path: &'a Path, target_path: &'a Path) -> Result<Self, Error> {
        let path_type_source = PathType::from(source_path);
        let target_state = TargetState::new(source_path, target_path)?;
        match (path_type_source, target_state) {
            (PathType::Directory, TargetState::Equals) => Ok(Self::Equals),
            (PathType::Directory, TargetState::NotPresent)
            | (PathType::Directory, TargetState::Occupied(PathType::Directory)) => Ok(Self::VacantDirectory {
                source_path,
                target_path,
            }),
            (PathType::Directory, TargetState::Occupied(PathType::File))
            | (PathType::Directory, TargetState::PointsTo(PathType::File))
            | (PathType::Directory, TargetState::PointsTo(PathType::Directory)) => {
                Err(Error::target_occupied(target_path))
            }
            (PathType::File, TargetState::Equals) => Ok(Self::Equals),
            (PathType::File, TargetState::NotPresent) => Ok(Self::VacantFile {
                source_path,
                target_path,
                target_exists: false,
            }),
            (PathType::File, TargetState::Occupied(PathType::Directory))
            | (PathType::File, TargetState::Occupied(PathType::File))
            | (PathType::File, TargetState::PointsTo(PathType::Directory)) => Err(Error::target_occupied(target_path)),
            (PathType::File, TargetState::PointsTo(PathType::File)) => Ok(Self::VacantFile {
                source_path,
                target_path,
                target_exists: true,
            }),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum TargetState {
    Equals,
    NotPresent,
    Occupied(PathType),
    PointsTo(PathType),
}

impl TargetState {
    fn new(source: &Path, target: &Path) -> Result<Self, Error> {
        if !target.exists() {
            Ok(Self::NotPresent)
        } else if target.is_symlink() {
            canonicalize(target)
                .map_err(|err| Error::canonicalize_target(err, target))
                .map(|real_target_path| {
                    if real_target_path == source {
                        TargetState::Equals
                    } else {
                        TargetState::PointsTo(PathType::from(real_target_path.as_ref()))
                    }
                })
        } else {
            Ok(Self::Occupied(PathType::from(target)))
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum PathType {
    Directory,
    File,
}

impl From<&Path> for PathType {
    fn from(value: &Path) -> Self {
        if value.is_dir() {
            Self::Directory
        } else {
            Self::File
        }
    }
}

fn create_directory(source: &Path, target: &Path) -> Result<(), Error> {
    if !target.exists() {
        create_dir(target).map_err(|err| Error::create_target_directory(err, target))?;
    }

    let source_entries = read_directory(source)?;
    for source_entry in source_entries {
        let source_entry = source_entry?;
        let source_entry_path = source_entry.path();
        let file_name = source_entry.file_name();
        let target_entry_path = target.join(file_name);
        create(source_entry_path, target_entry_path)?;
    }
    Ok(())
}

fn create_file(source: &Path, target: &Path) -> Result<(), Error> {
    if let Some(parent) = target.parent() {
        if !parent.exists() {
            create_dir_all(parent).map_err(|err| Error::create_parent(err, target))?;
        }
    }
    symlink(source, target).map_err(|err| Error::create_new_symlink(err, source, target))
}

fn remove_symlink(path: &Path) -> Result<(), Error> {
    remove_file(path).map_err(|err| Error::unlink(err, path))
}

fn remove_directory_entries(source: &Path, target: &Path) -> Result<(), Error> {
    if !target.exists() {
        return Ok(());
    }
    let target_entries = read_directory(target)?;
    for target_entry in target_entries {
        let target_entry = target_entry?;
        let target_entry_path = target_entry.path();
        if target_entry_path.is_symlink() {
            let real_path =
                canonicalize(&target_entry_path).map_err(|err| Error::canonicalize_target(err, &target_entry_path))?;
            if real_path.starts_with(source) {
                remove(real_path, &target_entry_path)?;
            }
        } else if target_entry_path.is_dir() {
            if let Ok(relative_target_path) = target_entry_path.strip_prefix(target) {
                let source_entry_path = source.join(relative_target_path);
                if source_entry_path.is_dir() {
                    remove_directory_entries(&source_entry_path, &target_entry_path)?;
                }
            }
        }
    }
    Ok(())
}

fn read_directory(path: &Path) -> Result<impl Iterator<Item = Result<DirEntry, Error>>, Error> {
    let path = path.to_owned();
    let iter = path.read_dir().map_err(|err| Error::read_directory(err, &path))?;
    Ok(iter.map(move |entry| entry.map_err(|err| Error::read_directory(err, &path))))
}

#[derive(Debug)]
pub enum Error {
    CanonicalizeTarget {
        err: io::Error,
        path: PathBuf,
    },
    CreateNewSymlink {
        err: io::Error,
        source: PathBuf,
        target: PathBuf,
    },
    CreateParent {
        err: io::Error,
        path: PathBuf,
    },
    CreateTargetDirectory {
        err: io::Error,
        path: PathBuf,
    },
    ReadDirectory {
        err: io::Error,
        path: PathBuf,
    },
    TargetOccupied(PathBuf),
    Unlink {
        err: io::Error,
        path: PathBuf,
    },
}

impl Error {
    fn canonicalize_target(err: io::Error, path: impl Into<PathBuf>) -> Self {
        Self::CanonicalizeTarget { err, path: path.into() }
    }

    fn create_new_symlink(err: io::Error, source: impl Into<PathBuf>, target: impl Into<PathBuf>) -> Self {
        Self::CreateNewSymlink {
            err,
            source: source.into(),
            target: target.into(),
        }
    }

    fn create_parent(err: io::Error, path: impl Into<PathBuf>) -> Self {
        Self::CreateParent { err, path: path.into() }
    }

    fn create_target_directory(err: io::Error, path: impl Into<PathBuf>) -> Self {
        Self::CreateTargetDirectory { err, path: path.into() }
    }

    fn read_directory(err: io::Error, path: impl Into<PathBuf>) -> Self {
        Self::ReadDirectory { err, path: path.into() }
    }

    fn target_occupied(path: impl Into<PathBuf>) -> Self {
        Self::TargetOccupied(path.into())
    }

    fn unlink(err: io::Error, path: impl Into<PathBuf>) -> Self {
        Self::Unlink { err, path: path.into() }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::CanonicalizeTarget { err, path } => write!(out, "canonicalize target: {}: {}", path.display(), err),
            Self::CreateNewSymlink { err, source, target } => write!(
                out,
                "create new symlink: {} -> {}: {}",
                source.display(),
                target.display(),
                err
            ),
            Self::CreateParent { err, path } => write!(out, "create parent directory for {}: {}", path.display(), err),
            Self::CreateTargetDirectory { err, path } => {
                write!(out, "create target directory: {}: {}", path.display(), err)
            }
            Self::ReadDirectory { err, path } => {
                write!(out, "read directory: {}: {}", path.display(), err)
            }
            Self::TargetOccupied(path) => write!(out, "target occupied: {}", path.display()),
            Self::Unlink { err, path } => write!(out, "unlink: {}: {}", path.display(), err),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(match self {
            Self::CanonicalizeTarget { err, .. } => err,
            Self::CreateNewSymlink { err, .. } => err,
            Self::CreateParent { err, .. } => err,
            Self::CreateTargetDirectory { err, .. } => err,
            Self::ReadDirectory { err, .. } => err,
            Self::TargetOccupied(_) => return None,
            Self::Unlink { err, .. } => err,
        })
    }
}
