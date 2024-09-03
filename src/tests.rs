use std::{
    error::Error,
    fs::{canonicalize, create_dir, read_to_string, write},
    os::unix::fs::symlink as create_symlink,
    path::{Path, PathBuf},
};

use tempfile::tempdir;

use crate::{command, handler, metadata};

#[test]
fn register_ok() {
    let root = tempdir().unwrap();
    let root_path = root.path().to_owned();
    let source_path = root_path.join("source-file");
    let source = source_path.to_str().unwrap().to_owned();
    let target = String::from("target-file");

    handler::register(command::ArgsRegister {
        metadata_path: root_path.join("makky.metadata").to_owned(),
        source: source.clone(),
        target: target.clone(),
    })
    .unwrap();

    let metadata = read_to_string(root_path.join("makky.metadata")).unwrap();
    assert_eq!(metadata, format!("{source}\n{target}\n"));
}

#[test]
fn register_new_entry_source_not_absolute() {
    let source = String::from("makky-source-file-not-found");
    let err = handler::register(command::ArgsRegister {
        metadata_path: PathBuf::from("/tmp/makky-config-root-not-found"),
        source: source.clone(),
        target: String::from("makky-target-file-not-found"),
    })
    .unwrap_err();
    assert!(err.source().is_some());
    assert_eq!(
        err.to_string(),
        format!("register: create new entry: new entry: source is not an absolute path: {source}")
    );
    if let handler::Error::RegisterNewEntryCreate(metadata::Error::NewEntrySourceNotAbsolute(path)) = err {
        assert_eq!(path.to_string_lossy(), source);
    } else {
        panic!("Unexpected error: {:?}", err)
    }
}

#[test]
fn register_new_entry_source_is_absolute() {
    let target = String::from("/tmp/makky-target-file-not-found");
    let err = handler::register(command::ArgsRegister {
        metadata_path: PathBuf::from("/tmp/makky-config-root-not-found"),
        source: String::from("/tmp/makky-source-file-not-found"),
        target: target.clone(),
    })
    .unwrap_err();
    assert!(err.source().is_some());
    assert_eq!(
        err.to_string(),
        format!("register: create new entry: new entry: target must be a relative path: {target}")
    );
    if let handler::Error::RegisterNewEntryCreate(metadata::Error::NewEntryTargetIsAbsolute(path)) = err {
        assert_eq!(path.to_string_lossy(), target);
    } else {
        panic!("Unexpected error: {:?}", err)
    }
}

struct LinkFile {
    source_path: PathBuf,
    source_content: String,
    target_path: PathBuf,
}

impl LinkFile {
    fn create(root_path: &Path, prefix: &str) -> LinkFile {
        let source_path = root_path.join(format!("{prefix}-file-source"));
        let source_content = format!("{prefix}-file-source-content");
        write(&source_path, &source_content).unwrap();
        let target_relative_path = format!("{prefix}-file-target");
        let target_path = root_path.join(&target_relative_path).to_owned();
        handler::register(command::ArgsRegister {
            metadata_path: root_path.join("makky.metadata").to_owned(),
            source: source_path.to_string_lossy().into_owned(),
            target: target_relative_path,
        })
        .unwrap();
        LinkFile {
            source_path: source_path.to_owned(),
            source_content,
            target_path,
        }
    }

    fn assert_target_created(&self) {
        assert!(self.target_path.exists());
        assert!(self.target_path.is_symlink());
        assert_symlink_equals(&self.source_path, &self.target_path);
        let target_content = read_to_string(&self.target_path).unwrap();
        assert_eq!(self.source_content, target_content);
    }

    fn assert_target_removed(&self) {
        assert!(!self.target_path.exists());
        assert!(self.source_path.exists());
    }
}

struct LinkDirectory {
    source_path: PathBuf,
    source_file_path: PathBuf,
    source_file_content: String,
    target_path: PathBuf,
    target_file_path: PathBuf,
}

impl LinkDirectory {
    fn create(root_path: &Path, directory_path: &Path, prefix: &str) -> LinkDirectory {
        let source_path = directory_path.join(format!("{prefix}-directory-source"));
        create_dir(&source_path).unwrap();
        let source_file_path = source_path.join("file");
        let source_file_content = format!("{prefix}-source-directory-file-content");
        write(&source_file_path, &source_file_content).unwrap();
        let target_relative_path = directory_path
            .strip_prefix(root_path)
            .unwrap()
            .join(format!("{prefix}-directory-target"))
            .to_string_lossy()
            .into_owned();
        let target_path = root_path.join(&target_relative_path).to_owned();
        let target_file_path = target_path.join("file").to_owned();
        handler::register(command::ArgsRegister {
            metadata_path: root_path.join("makky.metadata").to_owned(),
            source: source_path.to_string_lossy().into_owned(),
            target: target_relative_path,
        })
        .unwrap();
        LinkDirectory {
            source_path: source_path.to_owned(),
            source_file_path: source_file_path.to_owned(),
            source_file_content,
            target_path,
            target_file_path,
        }
    }

    fn assert_target_created(&self) {
        assert!(self.target_path.exists());
        assert!(!self.target_path.is_symlink());
        assert!(self.target_path.is_dir());
        assert!(self.target_file_path.exists());
        assert!(self.target_file_path.is_symlink());
        assert!(self.target_file_path.is_file());
        assert_symlink_equals(&self.source_file_path, &self.target_file_path);
        let target_file_content = read_to_string(&self.target_file_path).unwrap();
        assert_eq!(self.source_file_content, target_file_content);
    }

    fn assert_target_removed(&self) {
        assert!(!self.target_file_path.exists());
        assert!(self.source_path.exists());
        assert!(self.source_file_path.exists());
    }
}

fn assert_symlink_equals(source_path: &Path, target_path: &Path) {
    assert_eq!(canonicalize(target_path).unwrap(), source_path);
}

#[test]
fn link_unlink_ok() {
    let root = tempdir().unwrap();
    let root_path = root.path().to_owned();

    let file_x_path = root_path.join("file-x");
    write(&file_x_path, "file-x").unwrap();

    let file_link_equals = LinkFile::create(&root_path, "equals");
    create_symlink(&file_link_equals.source_path, &file_link_equals.target_path).unwrap();

    let file_link_vacant_not_present = LinkFile::create(&root_path, "vacant-not-present");

    let file_link_vacant_present = LinkFile::create(&root_path, "vacant-present");
    create_symlink(&file_x_path, &file_link_vacant_present.target_path).unwrap();

    let directory_link_equals = LinkDirectory::create(&root_path, &root_path, "equals");
    create_symlink(&directory_link_equals.source_path, &directory_link_equals.target_path).unwrap();

    let directory_link_level_0 = LinkDirectory::create(&root_path, &root_path, "level-0");
    let directory_link_level_1 = LinkDirectory::create(&root_path, &directory_link_level_0.source_path, "level-1");

    let directory_link_vacant_equals = LinkDirectory::create(&root_path, &root_path, "vacant-equals");
    create_dir(&directory_link_vacant_equals.target_path).unwrap();
    create_symlink(
        &directory_link_vacant_equals.source_file_path,
        &directory_link_vacant_equals.target_file_path,
    )
    .unwrap();

    let directory_link_vacant_not_present = LinkDirectory::create(&root_path, &root_path, "vacant-not-present");

    let directory_link_vacant_present = LinkDirectory::create(&root_path, &root_path, "vacant-present");
    create_dir(&directory_link_vacant_present.target_path).unwrap();
    create_symlink(&file_x_path, &directory_link_vacant_present.target_file_path).unwrap();

    let metadata_path = root_path.join("makky.metadata");

    for _ in 0..2 {
        handler::link(command::ArgsLink {
            metadata_path: metadata_path.clone(),
            target_root: root_path.clone(),
        })
        .unwrap();
    }

    assert_symlink_equals(&file_link_equals.source_path, &file_link_equals.target_path);
    file_link_vacant_not_present.assert_target_created();
    file_link_vacant_present.assert_target_created();
    assert_symlink_equals(&directory_link_equals.source_path, &directory_link_equals.target_path);
    directory_link_level_0.assert_target_created();
    directory_link_level_1.assert_target_created();
    directory_link_vacant_equals.assert_target_created();
    directory_link_vacant_not_present.assert_target_created();
    directory_link_vacant_present.assert_target_created();

    let mut entries: Vec<String> = root_path
        .read_dir()
        .unwrap()
        .map(|x| x.unwrap().path().to_string_lossy().into_owned())
        .collect();
    assert_eq!(entries.len(), 18);
    entries.sort();
    let mut expected: Vec<&Path> = vec![
        directory_link_equals.source_path.as_ref(),
        directory_link_equals.target_path.as_ref(),
        directory_link_level_0.source_path.as_ref(),
        directory_link_level_0.target_path.as_ref(),
        directory_link_vacant_equals.source_path.as_ref(),
        directory_link_vacant_equals.target_path.as_ref(),
        directory_link_vacant_not_present.source_path.as_ref(),
        directory_link_vacant_not_present.target_path.as_ref(),
        directory_link_vacant_present.source_path.as_ref(),
        directory_link_vacant_present.target_path.as_ref(),
        file_link_equals.source_path.as_ref(),
        file_link_equals.target_path.as_ref(),
        file_link_vacant_not_present.source_path.as_ref(),
        file_link_vacant_not_present.target_path.as_ref(),
        file_link_vacant_present.source_path.as_ref(),
        file_link_vacant_present.target_path.as_ref(),
        file_x_path.as_ref(),
        metadata_path.as_ref(),
    ];
    expected.sort();
    let expected = expected.into_iter().enumerate();
    for (idx, path) in expected {
        assert_eq!(entries[idx], path.to_string_lossy());
    }

    handler::unlink(command::ArgsUnlink {
        metadata_path: metadata_path.clone(),
        target_root: root_path,
    })
    .unwrap();

    file_link_equals.assert_target_removed();
    file_link_vacant_not_present.assert_target_removed();
    file_link_vacant_present.assert_target_removed();
    directory_link_equals.assert_target_removed();
    directory_link_level_0.assert_target_removed();
    directory_link_level_1.assert_target_removed();
    directory_link_vacant_equals.assert_target_removed();
    directory_link_vacant_not_present.assert_target_removed();
    directory_link_vacant_present.assert_target_removed();
}

#[test]
fn link_entry_source_not_exists() {
    let root = tempdir().unwrap();
    let root_path = root.path().to_owned();

    let source_path = root_path.join("not-exists-source");
    let metadata_path = root_path.join("makky.metadata").to_owned();

    handler::register(command::ArgsRegister {
        metadata_path: metadata_path.clone(),
        source: source_path.to_string_lossy().into_owned(),
        target: String::from("not-exists-target"),
    })
    .unwrap();

    let err = handler::link(command::ArgsLink {
        metadata_path: metadata_path.clone(),
        target_root: root_path.clone(),
    })
    .unwrap_err();

    assert!(err.source().is_some());

    assert_eq!(
        err.to_string(),
        format!(
            "link: read metadata: parse entries:\n\tentry: source not exists: {}",
            source_path.display()
        )
    );
    if let handler::Error::LinkReadMetadata(metadata::Error::ParseEntries(errors)) = err {
        assert_eq!(errors.len(), 1);
        let entry_error = &errors[0];
        assert!(entry_error.source().is_none());
        if let metadata::Error::EntrySourceNotExists(path) = entry_error {
            assert_eq!(path, &source_path);
        } else {
            panic!("Unexpected entry error: {:?}", entry_error);
        }
    } else {
        panic!("Unexpected error: {:?}", err);
    }
}

#[test]
fn link_entry_target_exists() {
    let root = tempdir().unwrap();
    let root_path = root.path().to_owned();

    let file_link_occupied = LinkFile::create(&root_path, "occupied");
    write(&file_link_occupied.target_path, "file-x").unwrap();

    let err = handler::link(command::ArgsLink {
        metadata_path: root_path.join("makky.metadata").to_owned(),
        target_root: root_path.clone(),
    })
    .unwrap_err();

    assert!(err.source().is_some());

    assert_eq!(
        err.to_string(),
        format!(
            "link: read metadata: parse entries:\n\tentry: target already exists: {}",
            file_link_occupied.target_path.display(),
        )
    );
    if let handler::Error::LinkReadMetadata(metadata::Error::ParseEntries(errors)) = err {
        assert_eq!(errors.len(), 1);
        let entry_error = &errors[0];
        assert!(entry_error.source().is_none());
        if let metadata::Error::EntryTargetExists(path) = entry_error {
            assert_eq!(path, &file_link_occupied.target_path);
        } else {
            panic!("Unexpected entry error: {:?}", entry_error);
        }
    } else {
        panic!("Unexpected error: {:?}", err);
    }
}

#[test]
fn link_entry_target_duplicate() {
    let root = tempdir().unwrap();
    let root_path = root.path().to_owned();

    let source_path = root_path.join("source").to_owned();
    write(&source_path, "source-content").unwrap();

    let metadata_path = root_path.join("makky.metadata").to_owned();

    for _ in 0..2 {
        handler::register(command::ArgsRegister {
            metadata_path: metadata_path.clone(),
            source: source_path.to_string_lossy().into_owned(),
            target: String::from("target"),
        })
        .unwrap();
    }

    let err = handler::link(command::ArgsLink {
        metadata_path: metadata_path.clone(),
        target_root: root_path,
    })
    .unwrap_err();
    assert!(err.source().is_some());
    assert_eq!(
        err.to_string(),
        format!(
            "link: read metadata: parse entries:\n\tentry: target duplicate: {} -> target",
            source_path.display()
        )
    );
    if let handler::Error::LinkReadMetadata(metadata_err) = err {
        assert!(metadata_err.source().is_none());
    } else {
        panic!("Unexpected error: {:?}", err);
    }
}

#[test]
fn link_invalid_target_root() {
    let err = handler::link(command::ArgsLink {
        metadata_path: PathBuf::from("/tmp/makky"),
        target_root: PathBuf::from("makky"),
    })
    .unwrap_err();
    assert!(err.source().is_some());
    assert_eq!(
        err.to_string(),
        "link: read metadata: target root is not an absolute path: makky"
    );

    let err = handler::link(command::ArgsLink {
        metadata_path: PathBuf::from("/tmp/makky"),
        target_root: PathBuf::from("/tmp/makky"),
    })
    .unwrap_err();
    assert!(err.source().is_some());
    assert_eq!(
        err.to_string(),
        "link: read metadata: target root is not a directory: /tmp/makky"
    );
}

#[test]
fn link_invalid_config() {
    let root = tempdir().unwrap();
    let root_path = root.path().to_owned();
    let metadata_path = root_path.join("makky.metadata").to_owned();

    let err = handler::link(command::ArgsLink {
        metadata_path: metadata_path.clone(),
        target_root: root_path.clone(),
    })
    .unwrap_err();
    assert!(err.source().is_some());
    assert_eq!(
        err.to_string(),
        "link: read metadata: open config: No such file or directory (os error 2)"
    );
    if let handler::Error::LinkReadMetadata(metadata_err) = err {
        assert!(metadata_err.source().is_some());
    } else {
        panic!("Unexpected error: {:?}", err);
    }

    let config_path = root_path.join("makky.metadata");
    write(&config_path, "x").unwrap();

    let err = handler::link(command::ArgsLink {
        metadata_path: metadata_path.clone(),
        target_root: root_path.clone(),
    })
    .unwrap_err();
    assert!(err.source().is_some());
    assert_eq!(err.to_string(), "link: read metadata: parse entry target: missing");
    if let handler::Error::LinkReadMetadata(metadata_err) = err {
        assert!(metadata_err.source().is_none());
    } else {
        panic!("Unexpected error: {:?}", err);
    }
}
