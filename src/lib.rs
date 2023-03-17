//! Library provides the struct [WithDir](crate::WithDir) which uses RAII
//! to enable scoped change of working directory. See docs for [WithDir](crate::WithDir)
//! for simple example.
use parking_lot::{ReentrantMutex, ReentrantMutexGuard};
use std::{
    env::{current_dir, set_current_dir},
    fs::{create_dir, create_dir_all},
    path::{Path, PathBuf},
};
use tempfile::TempDir;

static DIR_MUTEX: ReentrantMutex<()> = ReentrantMutex::new(());

enum Cwd {
    Temp(TempDir),
    NotTemp(PathBuf),
}

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
///
/// // create a directory
/// let path = std::env::current_dir().unwrap().join("a");
/// if !path.exists() {
///     std::fs::create_dir(&path);
/// }
///
/// // enter that directory
/// WithDir::new(&path).map( |_| {
///     assert_eq!(std::env::current_dir().unwrap(), path);
/// }).unwrap();
///
/// // cwd is reset
///
/// // enter it again
/// let cwd = WithDir::new("a").unwrap();
/// // exit it
/// cwd.leave().unwrap();
/// ```
///
///
pub struct WithDir<'a> {
    original_dir: PathBuf,
    cwd: Cwd,
    mutex: Option<ReentrantMutexGuard<'a, ()>>,
}

impl<'a> WithDir<'a> {
    /// On creation, the current working directory is set to `path`
    /// and a [ReentrantMutexGuard](parking_lot::ReentrantMutexGuard) is claimed.
    pub fn new(path: impl AsRef<Path>) -> Result<WithDir<'a>, std::io::Error> {
        let m = DIR_MUTEX.lock();
        let original_dir = current_dir()?;
        set_current_dir(&path)?;
        Ok(WithDir {
            original_dir,
            cwd: Cwd::NotTemp(path.as_ref().to_owned()),
            mutex: Some(m),
        })
    }

    /// Uses [TempDir](tempfile::TempDir) to create a temporary
    /// directory that with the same lifetime as the returned
    /// `WithDir`. The current working dir is change to the temp_dir
    pub fn temp() -> Result<WithDir<'a>, std::io::Error> {
        let m = DIR_MUTEX.lock();
        let original_dir = current_dir()?;
        let temp_dir = TempDir::new()?;
        set_current_dir(temp_dir.path())?;
        Ok(WithDir {
            original_dir,
            cwd: Cwd::Temp(temp_dir),
            mutex: Some(m),
        })
    }

    /// Makes a directory and changes the current working dir to that directory,
    /// the directory will persist after this `WithDir` is dropped. Use
    /// [create_all](crate::WithDir::create_all) if you want to also make the parent directories
    pub fn create(path: impl AsRef<Path>) -> Result<WithDir<'a>, std::io::Error> {
        let m = DIR_MUTEX.lock();
        let original_dir = current_dir()?;
        create_dir(&path)?;
        set_current_dir(&path)?;
        Ok(WithDir {
            original_dir,
            cwd: Cwd::NotTemp(path.as_ref().to_path_buf()),
            mutex: Some(m),
        })
    }

    /// See [create](crate::WithDir::create) for docs
    pub fn create_all(path: impl AsRef<Path>) -> Result<WithDir<'a>, std::io::Error> {
        let m = DIR_MUTEX.lock();
        let original_dir = current_dir()?;
        create_dir_all(&path)?;
        set_current_dir(&path)?;
        Ok(WithDir {
            original_dir,
            cwd: Cwd::NotTemp(path.as_ref().to_path_buf()),
            mutex: Some(m),
        })
    }

    /// Get that path that was changed to when this instance
    /// was created
    pub fn path(&self) -> &Path {
        match &self.cwd {
            Cwd::NotTemp(p) => p,
            Cwd::Temp(p) => p.path(),
        }
    }

    fn reset_cwd(&self) -> Result<(), std::io::Error> {
        set_current_dir(&self.original_dir)
    }

    /// Return to original working directory. This is exactly the
    /// same as dropping the instance but will not panic.
    pub fn leave(mut self) -> Result<(), std::io::Error> {
        let ret = self.reset_cwd();
        self.mutex = None;
        ret
    }
}

impl AsRef<Path> for WithDir<'_> {
    /// Returns the current working directory that was set when this
    /// instance was created.
    fn as_ref(&self) -> &Path {
        self.path()
    }
}

impl Drop for WithDir<'_> {
    /// Resets current working directory to whatever it was
    /// when this instance was created.
    ///
    /// # Panics
    ///
    /// Panics if the original directory is no longer accesible (has been deleted, etc.)
    fn drop(&mut self) {
        if self.mutex.is_some() {
            self.reset_cwd().unwrap();
        }
    }
}

// test the code in the readme
#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDoctests;

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
                WithDir::new(&b)
                    .map(|wd| {
                        let cwd = current_dir().unwrap();
                        assert_eq!(cwd, wd.path());
                    })
                    .unwrap();
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

                // test leave
                let wd = WithDir::new(&b).unwrap();
                let cwd = current_dir().unwrap();
                assert_eq!(cwd, wd.path());
                wd.leave().unwrap();

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

    #[test]
    fn test_create_dir() {
        let cwd = current_dir().unwrap();
        WithDir::create_all(cwd.join("a/create"))
            .map(|new_dir| {
                assert_eq!(current_dir().unwrap(), new_dir.path());

                WithDir::create(cwd.join("a/create/b"))
                    .map(|new_dir| {
                        assert_eq!(current_dir().unwrap(), new_dir.path());
                    })
                    .unwrap();
            })
            .unwrap();

        assert_eq!(cwd, current_dir().unwrap());
        assert!(cwd.join("a/create/b").exists());
    }

    #[test]
    fn test_temp_dir() {
        let cwd = current_dir().unwrap();
        let mut dir: Option<PathBuf> = None;

        WithDir::temp()
            .map(|d| {
                // path we changed to != original path
                assert_ne!(d.path(), cwd);
                dir = Some(current_dir().unwrap());
            })
            .unwrap();

        // temp dir was deleted
        assert!(!dir.unwrap().exists());
    }
}
