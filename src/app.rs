use std::{
    error,
    fmt,
    process::{ExitCode, Termination},
};

use crate::{command, handler};

#[derive(Debug)]
pub enum Error {
    Command(command::Error),
    Handler(handler::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Command(err) => write!(out, "command: {err}"),
            Self::Handler(err) => write!(out, "handler: {err}"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(match self {
            Self::Command(err) => err,
            Self::Handler(err) => err,
        })
    }
}

impl From<command::Error> for Error {
    fn from(value: command::Error) -> Self {
        Self::Command(value)
    }
}

impl From<handler::Error> for Error {
    fn from(value: handler::Error) -> Self {
        Self::Handler(value)
    }
}

pub enum Status {
    Ok,
    Err(Error),
}

impl Termination for Status {
    fn report(self) -> ExitCode {
        match self {
            Self::Ok => ExitCode::SUCCESS,
            Self::Err(err) => {
                eprintln!("Error: {err}");
                ExitCode::FAILURE
            }
        }
    }
}

fn execute() -> Result<(), Error> {
    match command::parse()? {
        command::Type::Link(args) => handler::link(args)?,
        command::Type::Register(args) => handler::register(args)?,
        command::Type::Unlink(args) => handler::unlink(args)?,
    }
    Ok(())
}

pub fn run() -> Status {
    match execute() {
        Ok(()) => Status::Ok,
        Err(err) => Status::Err(err),
    }
}
