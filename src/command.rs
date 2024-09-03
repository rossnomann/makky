use std::{error, fmt, path::PathBuf, str::FromStr};

#[derive(Debug)]
pub enum Type {
    Link(ArgsLink),
    Register(ArgsRegister),
    Unlink(ArgsUnlink),
}

#[derive(Debug)]
pub struct ArgsLink {
    pub metadata_path: PathBuf,
    pub target_root: PathBuf,
}

#[derive(Debug)]
pub struct ArgsRegister {
    pub metadata_path: PathBuf,
    pub source: String,
    pub target: String,
}

#[derive(Debug)]
pub struct ArgsUnlink {
    pub metadata_path: PathBuf,
    pub target_root: PathBuf,
}

#[derive(Clone, Copy, Debug)]
enum Name {
    Link,
    Register,
    Unlink,
}

impl FromStr for Name {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "link" => Self::Link,
            "register" => Self::Register,
            "unlink" => Self::Unlink,
            _ => return Err(Error::UnknownCommand(String::from(s))),
        })
    }
}

pub fn parse() -> Result<Type, Error> {
    let mut args = std::env::args().skip(1);
    let raw_name = args.next().ok_or(Error::CommandNotProvided)?;
    let name = raw_name.parse::<Name>()?;
    let raw_metadata_path = args.next().ok_or(Error::MetadataPathNotProvided)?;
    let metadata_path = PathBuf::from(raw_metadata_path);
    Ok(match name {
        Name::Link => {
            let raw_target_root = args.next().ok_or(Error::TargetRootNotProvided)?;
            let target_root = PathBuf::from(raw_target_root);
            Type::Link(ArgsLink {
                metadata_path,
                target_root,
            })
        }
        Name::Register => {
            let source = args.next().ok_or(Error::LinkSourceNotProvided)?;
            let target = args.next().ok_or(Error::LinkTargetNotProvided)?;
            Type::Register(ArgsRegister {
                metadata_path,
                source,
                target,
            })
        }
        Name::Unlink => {
            let raw_target_root = args.next().ok_or(Error::TargetRootNotProvided)?;
            let target_root = PathBuf::from(raw_target_root);
            Type::Unlink(ArgsUnlink {
                metadata_path,
                target_root,
            })
        }
    })
}

#[derive(Debug)]
pub enum Error {
    CommandNotProvided,
    LinkSourceNotProvided,
    LinkTargetNotProvided,
    MetadataPathNotProvided,
    TargetRootNotProvided,
    UnknownCommand(String),
}

impl fmt::Display for Error {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::CommandNotProvided => write!(out, "command not provided"),
            Self::LinkSourceNotProvided => write!(out, "link source not provided"),
            Self::LinkTargetNotProvided => write!(out, "link target not provided"),
            Self::MetadataPathNotProvided => write!(out, "metadata path not provided"),
            Self::TargetRootNotProvided => write!(out, "target root not provided"),
            Self::UnknownCommand(value) => write!(out, "unknown command: {value}"),
        }
    }
}

impl error::Error for Error {}
