extern crate racer;
extern crate tempfile;
use racer::{complete_from_file, find_definition, Match, Point};
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::thread;
use tempfile::{Builder, NamedTempFile, TempDir};

pub fn tmpname() -> String {
    let thread = thread::current();
    let taskname = thread.name().unwrap();
    let taskname = taskname.replace("::", "-");
    format!("racer-{}", taskname)
}

/// Utility wrapper for NamedTempfile
pub struct TmpFile {
    inner: NamedTempFile,
}

impl TmpFile {
    pub fn new(src: &str) -> Self {
        let name = tmpname();
        let mut file = Builder::new()
            .prefix(&name)
            .rand_bytes(0)
            .tempfile()
            .expect("failed to create tempfile.");
        file.as_file_mut()
            .write_all(src.as_bytes())
            .expect("couldn't write to temp file");
        TmpFile { inner: file }
    }
    pub fn path(&self) -> &Path {
        self.inner.path()
    }
}

impl fmt::Debug for TmpFile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TmpFile: {:?}", self.path())
    }
}

impl AsRef<Path> for TmpFile {
    fn as_ref(&self) -> &Path {
        self.path()
    }
}

/// Utility wrapper for TempDir
pub enum TmpDir {
    Tmp(TempDir),
    Real(PathBuf),
}

impl TmpDir {
    pub fn new() -> Self {
        let name = tmpname();
        let dir = Builder::new()
            .prefix(&name)
            .tempdir()
            .expect("failed to create tempdir.");
        TmpDir::Tmp(dir)
    }
    pub fn nested_dir(&self, dir_name: &str) -> Self {
        let path = self.path();
        let new_path = path.join(dir_name);
        if new_path.exists() {
            TmpDir::Real(new_path)
        } else {
            let dir = Builder::new()
                .prefix(&dir_name)
                .rand_bytes(0)
                .tempdir_in(path)
                .expect("failed to create nested tempdir.");
            TmpDir::Tmp(dir)
        }
    }
    pub fn write_file(&self, file_name: &str, src: &str) -> TmpFile {
        let path = self.path();
        let mut file = Builder::new()
            .prefix(file_name)
            .rand_bytes(0)
            .tempfile_in(path)
            .expect("failed to create nested tempfile.");
        file.as_file_mut()
            .write_all(src.as_bytes())
            .expect("couldn't write to temp file");
        TmpFile { inner: file }
    }
    pub fn path(&self) -> &Path {
        match self {
            TmpDir::Tmp(dir) => dir.path(),
            TmpDir::Real(buf) => buf,
        }
    }
}

impl AsRef<Path> for TmpDir {
    fn as_ref(&self) -> &Path {
        self.path()
    }
}

impl fmt::Debug for TmpDir {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TmpDir: {:?}", self.path())
    }
}

/// copy test_project/* into TempDir
pub fn setup_test_project() -> TmpDir {
    let tmp_dir = TmpDir::new();
    let test_project = Path::new(env!("CARGO_MANIFEST_DIR")).join("test_project");
    fn copy_dirs(abs_path: &Path, tmp_path: &Path) -> io::Result<()> {
        if !abs_path.is_dir() {
            return Ok(());
        }
        for entry in fs::read_dir(abs_path)? {
            let entry = entry?;
            let path = entry.path();
            let relative = path.strip_prefix(abs_path)
                .expect("[setup_test_project] failed to strip_prefix(bug)");
            let relative = relative
                .to_str()
                .expect("[setup_test_project] failed to get &str from path");
            if relative == "target" {
                continue;
            }
            let nxt_tmp_path = tmp_path.join(relative);
            if path.is_dir() {
                fs::create_dir(&nxt_tmp_path)?;
                copy_dirs(&path, &nxt_tmp_path)?;
            } else {
                fs::File::create(&nxt_tmp_path)?;
                fs::copy(&path, &nxt_tmp_path)?;
            }
        }
        Ok(())
    }
    copy_dirs(&test_project, tmp_dir.path()).unwrap();
    tmp_dir
}

pub fn get_pos_and_source(src: &str) -> (Point, String) {
    let point = src.find('~').unwrap();
    (point, src.replace('~', ""))
}

/// Return the completions for the given source.
///
/// The point to find completions at must be marked with '~'.
pub fn get_all_completions(src: &str, dir: Option<TmpDir>) -> Vec<Match> {
    let dir = dir.unwrap_or_else(|| TmpDir::new());
    let (completion_point, clean_src) = get_pos_and_source(src);
    let path = dir.write_file("src.rs", &clean_src);
    let cache = racer::FileCache::default();
    let session = racer::Session::new(&cache);
    complete_from_file(&path, completion_point, &session).collect()
}

/// Return the first completion for the given source.
pub fn get_one_completion(src: &str, dir: Option<TmpDir>) -> Match {
    get_all_completions(src, dir).swap_remove(0)
}

/// Return the first completion for the given source, which must be
/// the only one.
///
/// # Panics
/// Panics if there is not exactly one completion.
pub fn get_only_completion(src: &str, dir: Option<TmpDir>) -> Match {
    let mut all = get_all_completions(src, dir);
    assert_eq!(all.len(), 1);
    all.pop().unwrap()
}

/// Return the definition for the given source.
///
/// The point to find the definition at must be marked with '~'.
pub fn get_definition(src: &str, dir: Option<TmpDir>) -> Match {
    let dir = dir.unwrap_or_else(|| TmpDir::new());
    let (completion_point, clean_src) = get_pos_and_source(src);
    let path = dir.write_file("src.rs", &clean_src);
    let cache = racer::FileCache::default();
    let session = racer::Session::new(&cache);
    find_definition(&path, completion_point, &session).unwrap()
}
