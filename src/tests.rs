use std::{error::Error, fs, os::unix::fs::symlink as create_symlink, path, process::Termination};

#[test]
fn err_cmd_missing() {
    let err = run(vec![]).unwrap_err();
    assert_eq!(err.to_string(), "command not provided");
    assert!(err.source().is_none());
}

#[test]
fn err_cmd_unknown() {
    let cmd = String::from("unknown");
    let err = run(vec![cmd.clone()]).unwrap_err();
    assert_eq!(err.to_string(), "unknown command: unknown");
    assert!(err.source().is_none());
}

#[test]
fn err_register_arguments_missing() {
    let err = run(vec![String::from("register")]).unwrap_err();
    assert_eq!(err.to_string(), "register: metadata path not provided");
    assert!(err.source().is_none());

    let err = run(vec![String::from("register"), String::from("metadata")]).unwrap_err();
    assert_eq!(err.to_string(), "register: link source not provided");
    assert!(err.source().is_none());

    let err = run(vec![
        String::from("register"),
        String::from("metadata"),
        String::from("source"),
    ])
    .unwrap_err();
    assert_eq!(err.to_string(), "register: link target not provided");
    assert!(err.source().is_none());
}

#[test]
fn err_register_source_not_absolute() {
    let source = path::PathBuf::from("makky-source-not-absolute");
    let err = execute_register(
        path::PathBuf::from("/tmp/makky-metadata-not-found"),
        &source,
        String::from("makky-target-file-not-found"),
    )
    .unwrap_err();
    let source = convert_path_to_string(source);
    assert_eq!(
        err.to_string(),
        format!("register: new entry: source is not an absolute path: {source}")
    );
    let err_source = err.source().unwrap();
    assert!(err_source.source().is_none());
}

#[test]
fn err_register_target_not_relative() {
    let target = String::from("/tmp/makky-target-file-not-found");
    let err = execute_register(
        path::PathBuf::from("/tmp/makky-metadata-not-found"),
        path::PathBuf::from("/tmp/makky-source-file-not-found"),
        target.clone(),
    )
    .unwrap_err();
    assert_eq!(
        err.to_string(),
        format!("register: new entry: target must be a relative path: {target}")
    );
    let err_source = err.source().unwrap();
    assert!(err_source.source().is_none());
}

#[test]
fn err_register_metadata_not_found() {
    let err = execute_register(
        path::PathBuf::from("/tmp/makky/metadata-not-found"),
        path::PathBuf::from("/tmp/makky-source-file-not-found"),
        String::from("target"),
    )
    .unwrap_err();
    assert_eq!(
        err.to_string(),
        format!("register: open config: No such file or directory (os error 2)")
    );
    let err_source = err.source().unwrap();
    assert!(err_source.source().is_some());
}

#[test]
fn err_register_metadata_readonly() {
    let context = Context::new();
    let metadata_store_path = context.metadata_store_path();
    let f = fs::File::create(&metadata_store_path).unwrap();
    let m = f.metadata().unwrap();
    let mut p = m.permissions();
    p.set_readonly(true);
    fs::set_permissions(&metadata_store_path, p).unwrap();
    let err = execute_register(
        &metadata_store_path,
        path::PathBuf::from("/tmp/makky-source-file-not-found"),
        String::from("target"),
    )
    .unwrap_err();
    assert_eq!(
        err.to_string(),
        format!("register: open config: Permission denied (os error 13)")
    );
    let err_source = err.source().unwrap();
    assert!(err_source.source().is_some());
}

#[test]
fn err_activate_arguments_missing() {
    let err = run(vec![String::from("activate")]).unwrap_err();
    assert_eq!(err.to_string(), "activate: actual metadata path not provided");
    assert!(err.source().is_none());

    let err = run(vec![String::from("activate"), String::from("metadata-actual")]).unwrap_err();
    assert_eq!(err.to_string(), "activate: store metadata path not provided");
    assert!(err.source().is_none());

    let err = run(vec![
        String::from("activate"),
        String::from("metadata-actual"),
        String::from("metadata-store"),
    ])
    .unwrap_err();
    assert_eq!(err.to_string(), "activate: target root not provided");
    assert!(err.source().is_none());
}

#[test]
fn err_activate_metadata_actual_is_dir() {
    let context = Context::new();
    let metadata_actual_path = context.metadata_actual_path();
    fs::create_dir(&metadata_actual_path).unwrap();
    let metadata_store_path = context.metadata_store_path();
    fs::write(&metadata_store_path, "test\ntest").unwrap();
    let err = execute_activate(&metadata_actual_path, &metadata_store_path, &context.target_root).unwrap_err();
    assert_eq!(
        err.to_string(),
        format!("activate: metadata: compare config: parse entry source: Is a directory (os error 21)")
    );
    let err_metadata = err.source().unwrap();
    let err_io = err_metadata.source().unwrap();
    let err_io_source = err_io.source().unwrap();
    assert!(err_io_source.source().is_none());
}

#[test]
fn err_activate_metadata_store_not_found() {
    let metadata_store_path = path::PathBuf::from("/tmp/makky/metadata/store");
    let err = execute_activate(
        path::PathBuf::from("/tmp/makky/metadata/actual"),
        &metadata_store_path,
        path::PathBuf::from("/tmp/makky/target/root"),
    )
    .unwrap_err();
    let metadata_store_path = convert_path_to_string(metadata_store_path);
    assert_eq!(
        err.to_string(),
        format!("activate: metadata store path is not found: {metadata_store_path}")
    );
    assert!(err.source().is_none());
}

#[test]
fn err_activate_metadata_store_invalid() {
    let context = Context::new();
    let metadata_store_path = context.metadata_store_path();
    fs::write(&metadata_store_path, "test").unwrap();
    let err = execute_activate(context.metadata_actual_path(), metadata_store_path, context.target_root).unwrap_err();
    assert_eq!(err.to_string(), "activate: metadata: parse entry target: missing");
    let err_source = err.source().unwrap();
    assert!(err_source.source().is_none());
}

#[test]
fn err_activate_metadata_copy_failed() {
    let context = Context::new();
    let metadata_actual_path = context.target_root.join("test/makky.metadata");
    let metadata_store_path = context.metadata_store_path();
    fs::write(&metadata_store_path, "test\ntest").unwrap();
    let err = execute_activate(&metadata_actual_path, &metadata_store_path, &context.target_root).unwrap_err();
    let metadata_actual_path = convert_path_to_string(metadata_actual_path);
    let metadata_store_path = convert_path_to_string(metadata_store_path);
    let expected_msg = format!("activate: metadata: copy config: {metadata_store_path} -> {metadata_actual_path}: No such file or directory (os error 2)");
    assert_eq!(err.to_string(), expected_msg);
    let err_source = err.source().unwrap();
    assert!(err_source.source().is_some());
}

#[test]
fn err_activate_target_root_not_a_directory() {
    let context = Context::new();
    let metadata_store_path = context.metadata_store_path();
    fs::write(&metadata_store_path, "test\ntest").unwrap();
    let target_root = context.target_root.join("test");
    let err = execute_activate(
        context.metadata_actual_path(),
        context.metadata_store_path(),
        &target_root,
    )
    .unwrap_err();
    let target_root = convert_path_to_string(target_root);
    assert_eq!(
        err.to_string(),
        format!("activate: metadata: target root is not a directory: {target_root}")
    );
    let err_source = err.source().unwrap();
    assert!(err_source.source().is_none());
}

#[test]
fn err_activate_target_root_not_absolute() {
    let context = Context::new();
    let metadata_store_path = context.metadata_store_path();
    fs::write(&metadata_store_path, "test\ntest").unwrap();
    let target_root = path::PathBuf::from("test");
    let err = execute_activate(
        context.metadata_actual_path(),
        context.metadata_store_path(),
        &target_root,
    )
    .unwrap_err();
    let target_root = convert_path_to_string(target_root);
    assert_eq!(
        err.to_string(),
        format!("activate: metadata: target root is not an absolute path: {target_root}")
    );
    let err_source = err.source().unwrap();
    assert!(err_source.source().is_none());
}

#[test]
fn err_activate_source_not_exists() {
    let context = Context::new();
    let metadata_store_path = context.metadata_store_path();
    fs::write(&metadata_store_path, "test\ntest").unwrap();
    let err = execute_activate(
        context.metadata_actual_path(),
        context.metadata_store_path(),
        &context.target_root,
    )
    .unwrap_err();
    assert_eq!(
        err.to_string(),
        "activate: metadata: parse entries:\n\tentry: source not exists: test",
    );
    let err_source = err.source().unwrap();
    assert!(err_source.source().is_none());
}

#[test]
fn err_activate_target_exists() {
    let context = Context::new();
    let metadata_store_path = context.metadata_store_path();
    let source = context.store_root.join("test");
    fs::write(
        &metadata_store_path,
        format!("{}\ntest", convert_path_to_string(&source)),
    )
    .unwrap();
    fs::write(source, "source-contents").unwrap();
    fs::write(context.target_root.join("test"), "test-exists").unwrap();
    let err = execute_activate(
        context.metadata_actual_path(),
        context.metadata_store_path(),
        &context.target_root,
    )
    .unwrap_err();
    let target = convert_path_to_string(context.target_root.join("test"));
    assert_eq!(
        err.to_string(),
        format!("activate: metadata: parse entries:\n\tentry: target already exists: {target}"),
    );
    let err_source = err.source().unwrap();
    assert!(err_source.source().is_none());
}

#[test]
fn err_activate_target_occupied() {
    let context = Context::new();
    let metadata_store_path = context.metadata_store_path();
    let source = context.store_root.join("test");
    fs::write(
        &metadata_store_path,
        format!("{}\ntest", convert_path_to_string(&source)),
    )
    .unwrap();
    fs::write(&source, "source-contents").unwrap();
    fs::create_dir(context.target_root.join("test")).unwrap();
    let err = execute_activate(
        context.metadata_actual_path(),
        context.metadata_store_path(),
        &context.target_root,
    )
    .unwrap_err();
    let source = convert_path_to_string(source);
    let target = convert_path_to_string(context.target_root.join("test"));
    assert_eq!(
        err.to_string(),
        format!("activate: create symlink {source} -> {target}: target occupied: {target}"),
    );
    assert!(err.source().is_none());
}

#[test]
fn err_activate_target_duplicate() {
    let context = Context::new();
    let metadata_store_path = context.metadata_store_path();
    let source = convert_path_to_string(context.store_root.join("test"));
    fs::write(&metadata_store_path, format!("{source}\ntest\n{source}\ntest")).unwrap();
    fs::write(&source, "source-contents").unwrap();
    let err = execute_activate(
        context.metadata_actual_path(),
        context.metadata_store_path(),
        &context.target_root,
    )
    .unwrap_err();
    assert_eq!(
        err.to_string(),
        format!("activate: metadata: parse entries:\n\tentry: target duplicate: {source} -> test"),
    );
    let err_source = err.source().unwrap();
    assert!(err_source.source().is_none());
}

#[test]
fn ok_default_args() {
    crate::App::default();
}

#[test]
fn ok_status_report() {
    let status = crate::Status::Ok;
    assert_eq!(format!("{:?}", status.report()), "ExitCode(unix_exit_status(0))");

    let status = crate::Status::Err(crate::Error::CmdMissing);
    assert_eq!(format!("{:?}", status.report()), "ExitCode(unix_exit_status(1))");
}

#[test]
fn ok() {
    let context = Context::new();
    let store_root = &context.store_root;

    let source_1 = store_root.join("source-1");
    fs::write(&source_1, "source-1_contents").unwrap();

    let source_2 = store_root.join("source-2");
    fs::write(&source_2, "source-2_contents").unwrap();
    create_symlink(&source_2, store_root.join("source-2-symlink")).unwrap();

    let source_3 = store_root.join("source-3");
    let source_3_file = source_3.join("file");
    let source_3_1 = source_3.join("1");
    let source_3_1_file = source_3_1.join("file");
    let source_3_symlink = store_root.join("source-3-symlink");
    fs::create_dir(&source_3).unwrap();
    fs::write(&source_3_file, "source-3_contents").unwrap();
    fs::create_dir(&source_3_1).unwrap();
    fs::write(&source_3_1_file, "source-3_1_contents").unwrap();
    create_symlink(&source_3, source_3_symlink).unwrap();

    let source_4 = store_root.join("source-4/1/2/3");
    let source_4_1_2_3_file = source_4.join("file");
    fs::create_dir_all(&source_4).unwrap();
    fs::write(&source_4_1_2_3_file, "source-4_1_2_3_contents").unwrap();

    let target_root = &context.target_root;
    fs::create_dir(target_root.join("target-1")).unwrap();
    fs::create_dir(target_root.join("target-2")).unwrap();
    fs::create_dir_all(target_root.join("target-3/1/2")).unwrap();
    fs::create_dir_all(target_root.join("target-4/1/2/3")).unwrap();

    let metadata_store_path = &context.metadata_store_path();
    for (source, target) in [
        (store_root.join("source-1"), String::from("target-1/file")),
        (store_root.join("source-2-symlink"), String::from("target-2/file")),
        (store_root.join("source-3"), String::from("target-3/1/2")),
        (store_root.join("source-4/1/2/3"), String::from("target-4/1/2/3")),
    ] {
        execute_register(metadata_store_path, source, target).unwrap();
    }

    assert_eq!(
        fs::read_to_string(metadata_store_path)
            .unwrap()
            .split("\n")
            .collect::<Vec<_>>(),
        [
            &convert_path_to_string(store_root.join("source-1")),
            "target-1/file",
            &convert_path_to_string(store_root.join("source-2-symlink")),
            "target-2/file",
            &convert_path_to_string(store_root.join("source-3")),
            "target-3/1/2",
            &convert_path_to_string(store_root.join("source-4/1/2/3")),
            "target-4/1/2/3",
            ""
        ]
    );

    let activate = || {
        execute_activate(
            context.metadata_actual_path(),
            context.metadata_store_path(),
            &context.target_root,
        )
        .unwrap();
    };

    let assert_activated = || {
        let target_root = &context.target_root;

        let target_1 = target_root.join("target-1");
        assert_dir_contains(&target_1, 1);

        let target_1_file = target_1.join("file");
        assert_symlink_contains(&target_1_file, &source_1, "source-1_contents");

        let target_2 = target_root.join("target-2");
        assert_dir_contains(&target_2, 1);

        let target_2_file = target_2.join("file");
        assert_symlink_contains(&target_2_file, &source_2, "source-2_contents");

        let target_3 = target_root.join("target-3");
        assert_dir_contains(&target_3, 1);

        let target_3_1 = target_3.join("1");
        assert_dir_contains(&target_3_1, 1);

        let target_3_1_2 = target_3_1.join("2");
        assert_dir_contains(&target_3_1_2, 2);

        let target_3_1_2_file = target_3_1_2.join("file");
        assert_symlink_contains(&target_3_1_2_file, &source_3_file, "source-3_contents");

        let target_3_1_2_1 = target_3_1_2.join("1");
        assert_dir_contains(&target_3_1_2_1, 1);

        let target_3_1_2_1_file = target_3_1_2_1.join("file");
        assert_symlink_contains(&target_3_1_2_1_file, &source_3_1_file, "source-3_1_contents");

        let target_4 = target_root.join("target-4");
        assert_dir_contains(&target_4, 1);

        let target_4_1 = target_4.join("1");
        assert_dir_contains(&target_4_1, 1);

        let target_4_1_2 = target_4_1.join("2");
        assert_dir_contains(&target_4_1_2, 1);

        let target_4_1_2_3 = target_4_1_2.join("3");
        assert_dir_contains(&target_4_1_2_3, 1);

        let target_4_1_2_3_file = target_4_1_2_3.join("file");
        assert_symlink_contains(&target_4_1_2_3_file, &source_4_1_2_3_file, "source-4_1_2_3_contents");
    };

    for _ in 1..3 {
        activate();
        assert_activated();
    }

    fs::remove_file(context.metadata_actual_path()).unwrap();
    activate();
    assert_activated();

    execute_register(
        metadata_store_path,
        convert_path_to_string(&source_1),
        String::from("new-target-1"),
    )
    .unwrap();
    activate();
    assert_activated();

    assert_symlink_contains(target_root.join("new-target-1"), source_1, "source-1_contents");
}

fn assert_dir_contains(path: impl AsRef<path::Path>, count: usize) {
    let path = path.as_ref();
    assert!(path.is_dir());
    assert_eq!(fs::read_dir(path).unwrap().count(), count);
}

fn assert_symlink_contains(source: impl AsRef<path::Path>, target: impl AsRef<path::Path>, contents: &str) {
    let source = source.as_ref();
    let target = target.as_ref();
    assert!(source.is_symlink());
    assert_eq!(fs::canonicalize(source).unwrap(), target);
    assert_eq!(fs::read_to_string(source).unwrap(), contents);
}

struct Context {
    _working_directory: tempfile::TempDir,
    store_root: path::PathBuf,
    target_root: path::PathBuf,
}

impl Context {
    fn new() -> Self {
        let working_directory = tempfile::tempdir().unwrap();
        let working_directory_path = working_directory.path();
        let store_root = working_directory_path.join("store");
        fs::create_dir(&store_root).unwrap();
        let target_root = working_directory_path.join("target");
        fs::create_dir(&target_root).unwrap();
        Self {
            _working_directory: working_directory,
            store_root,
            target_root,
        }
    }

    fn metadata_store_path(&self) -> path::PathBuf {
        self.store_root.join("makky.metadata").to_path_buf()
    }

    fn metadata_actual_path(&self) -> path::PathBuf {
        self.target_root.join("makky.metadata").to_path_buf()
    }
}

fn execute_register(
    metadata_path: impl AsRef<path::Path>,
    source: impl AsRef<path::Path>,
    target: String,
) -> Result<(), crate::Error> {
    run(vec![
        String::from("register"),
        convert_path_to_string(metadata_path),
        convert_path_to_string(source),
        target,
    ])
}

fn execute_activate(
    metadata_actual_path: impl AsRef<path::Path>,
    metadata_store_path: impl AsRef<path::Path>,
    target_root: impl AsRef<path::Path>,
) -> Result<(), crate::Error> {
    run(vec![
        String::from("activate"),
        convert_path_to_string(metadata_actual_path),
        convert_path_to_string(metadata_store_path),
        convert_path_to_string(target_root),
    ])
}

fn run(args: Vec<String>) -> Result<(), crate::Error> {
    crate::App::from(args.into_iter()).run().into()
}

fn convert_path_to_string(path: impl AsRef<path::Path>) -> String {
    path.as_ref().as_os_str().to_str().unwrap().to_string()
}
