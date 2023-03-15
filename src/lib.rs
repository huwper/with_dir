//! Library provides the struct [WithDir](crate::WithDir) which uses RAII
//! to enable scoped change of working directory. See docs for [WithDir](crate::WithDir)
//! for simple example.
use parking_lot::{ReentrantMutex, ReentrantMutexGuard};
use std::{
    env::{current_dir, set_current_dir},
    marker::PhantomData,
    path::{Path, PathBuf},
};

static DIR_MUTEX: ReentrantMutex<PhantomData<u8>> = ReentrantMutex::new(PhantomData);

/// Scoped modifier of the current working directory. This uses RAII to set the
/// current working directory back to what it was when the instance is dropped.
/// This struct uses a static `parking_lot::ReentrantMutex` to prevent `WithDir` on other
/// threads from updating the current working directory while any WithDir instances
/// exist. However there is nothing stopping other threads from calling `std::env::set_current_dir`
/// directly which would override the working directory.
///
/// WithDir should be created with `new` which returns a result. Result couldbe Err if the
/// directory doesn't exist, or if the user does not have permission to access.
///
/// ```
/// use with_dir::WithDir;
/// use tempdir::TempDir;
/// use std::env::current_dir;
///
/// // create a directory
/// let dir = TempDir::new("demo").unwrap();
/// let path = dir.path();
///
/// // enter that directory
/// if let Ok(cwd) = WithDir::new(path) {
///     // Current working directory is now path
/// };
///
/// // cwd is reset
/// ```
pub struct WithDir<'a> {
    original_dir: PathBuf,
    cwd: PathBuf,
    _mutex: ReentrantMutexGuard<'a, PhantomData<u8>>,
}

impl<'a> WithDir<'a> {
    /// On creation, the current working directory is set to `path`
    /// and a `parking_lot::ReentrantMutexGuard` is claimed.
    pub fn new(path: &Path) -> Result<WithDir, std::io::Error> {
        let m = DIR_MUTEX.lock();
        let original_dir = current_dir()?;
        set_current_dir(path)?;
        Ok(WithDir {
            original_dir,
            cwd: path.to_path_buf(),
            _mutex: m,
        })
    }

    /// Get that path that was changed to when this instance
    /// was created
    pub fn path(&self) -> &Path {
        &self.cwd
    }
}

impl<'a> Drop for WithDir<'a> {
    /// Resets current working directory to whatever it was
    /// when this instance was created.
    fn drop(&mut self) {
        set_current_dir(&self.original_dir).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::create_dir_all, thread};

    use super::*;

    #[test]
    fn it_works() {
        let cwd = current_dir().unwrap();
        let a = cwd.join("a");
        let b = a.join("b");

        if !b.exists() {
            create_dir_all(&b).unwrap();
        }

        match WithDir::new(&a) {
            Ok(_) => {
                let cwd = current_dir().unwrap();
                assert_eq!(cwd, a);
                {
                    let wd = WithDir::new(&b).unwrap();
                    let cwd = current_dir().unwrap();
                    assert_eq!(cwd, wd.path());
                };
                let cwd = current_dir().unwrap();
                assert_eq!(cwd, a);
            }
            Err(e) => {
                println!("{:?}", e);
                panic!("failed");
            }
        };
    }

    fn threaded_test_worker(p: &Path) {
        let a = p.join("a");
        let b = a.join("b");

        if !b.exists() {
            create_dir_all(&b).unwrap();
        }

        match WithDir::new(&a) {
            Ok(_) => {
                let cwd = current_dir().unwrap();
                assert_eq!(cwd, a);

                {
                    let wd = WithDir::new(&b).unwrap();
                    let cwd = current_dir().unwrap();
                    assert_eq!(cwd, wd.path());
                };

                let cwd = current_dir().unwrap();
                assert_eq!(cwd, a);
            }
            Err(e) => panic!("{}", e),
        };
    }

    #[test]
    fn test_multithreaded() {
        let cwd = current_dir().unwrap();
        let p1 = cwd.join("a/1");
        let p2 = cwd.join("a/2");
        let t1 = thread::spawn(move || threaded_test_worker(&p1));
        let t2 = thread::spawn(move || threaded_test_worker(&p2));
        t1.join().unwrap();
        t2.join().unwrap();
    }
}
