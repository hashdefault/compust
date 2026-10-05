use rustix::{
    fd::{AsFd, BorrowedFd, OwnedFd},
    fs::inotify::{self, CreateFlags, Event, ReadFlags, WatchFlags},
    io::Errno,
};
use std::{
    ffi::OsString,
    mem::MaybeUninit,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

/// How long the watched files must stay unchanged before a change reloads them. Editors save
/// in steps, such as moving the old file aside before writing the new one, and a reload
/// between two steps would read no file at all.
const SETTLE: Duration = Duration::from_millis(100);

/// What happens in a watched directory that can change a watched file, or the directory.
const WATCHED: WatchFlags = WatchFlags::CLOSE_WRITE
    .union(WatchFlags::MODIFY)
    .union(WatchFlags::CREATE)
    .union(WatchFlags::DELETE)
    .union(WatchFlags::MOVED_FROM)
    .union(WatchFlags::MOVED_TO)
    .union(WatchFlags::DELETE_SELF)
    .union(WatchFlags::MOVE_SELF)
    .union(WatchFlags::ONLYDIR);

/// Watches the files a configuration reload reads, through the directories that hold them:
/// many editors save a file by replacing it, which a watch on the file itself would lose.
pub(crate) struct Watch {
    fd: OwnedFd,
    /// Each watched directory, with the names in it that a reload reads.
    directories: Vec<(i32, Vec<OsString>)>,
    /// When the last change will have settled and the configuration is due to reload.
    due: Option<Instant>,
}

impl Watch {
    pub(crate) fn new() -> rustix::io::Result<Self> {
        Ok(Self {
            fd: inotify::init(CreateFlags::CLOEXEC | CreateFlags::NONBLOCK)?,
            directories: Vec::new(),
            due: None,
        })
    }

    /// Watch `files` instead of the files watched before. A directory that does not exist is
    /// left out, so a file created in it later is read on SIGUSR1 or a restart.
    pub(crate) fn follow(&mut self, files: &[PathBuf]) {
        let mut directories: Vec<(i32, Vec<OsString>)> = Vec::new();
        for file in files {
            let (Some(parent), Some(name)) = (file.parent(), file.file_name()) else {
                continue;
            };
            let directory = if parent.as_os_str().is_empty() {
                Path::new(".")
            } else {
                parent
            };
            // A directory watched already, under this name or another, keeps its descriptor.
            match inotify::add_watch(&self.fd, directory, WATCHED) {
                Ok(descriptor) => {
                    match directories
                        .iter_mut()
                        .find(|(known, _)| *known == descriptor)
                    {
                        Some((_, names)) => names.push(name.to_owned()),
                        None => directories.push((descriptor, vec![name.to_owned()])),
                    }
                }
                Err(Errno::NOENT | Errno::NOTDIR) => (),
                Err(error) => tracing::warn!(
                    directory = %directory.display(),
                    "not watching for configuration changes: {error}"
                ),
            }
        }
        for (descriptor, _) in &self.directories {
            if !directories.iter().any(|(kept, _)| kept == descriptor)
                && let Err(error) = inotify::remove_watch(&self.fd, *descriptor)
            {
                // A directory that went away took its watch with it.
                tracing::debug!("configuration watch already gone: {error}");
            }
        }
        self.directories = directories;
    }

    /// Read what changed by `now`. A change to a watched file makes the reload due once the
    /// files have stayed unchanged for a moment, which each later change postpones.
    pub(crate) fn read(&mut self, now: Instant) -> rustix::io::Result<()> {
        let mut buffer = [MaybeUninit::uninit(); 4096];
        let mut reader = inotify::Reader::new(&self.fd, &mut buffer);
        loop {
            match reader.next() {
                Ok(event) => {
                    if concerns(&self.directories, &event) {
                        self.due = Some(now + SETTLE);
                    }
                }
                Err(Errno::AGAIN) => return Ok(()),
                Err(error) => return Err(error),
            }
        }
    }

    /// When the configuration is due to reload, while a change has yet to settle.
    pub(crate) fn due(&self) -> Option<Instant> {
        self.due
    }

    /// Whether a change has settled by `now`, which takes it as reloaded.
    pub(crate) fn settled(&mut self, now: Instant) -> bool {
        let settled = self.due.is_some_and(|due| now >= due);
        if settled {
            self.due = None;
        }
        settled
    }
}

impl AsFd for Watch {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

/// Whether `event` can change what a reload reads: it names a watched file, its directory went
/// away, or the kernel dropped events.
fn concerns(directories: &[(i32, Vec<OsString>)], event: &Event<'_>) -> bool {
    let flags = event.events();
    if flags.contains(ReadFlags::QUEUE_OVERFLOW) {
        return true;
    }
    let Some((_, names)) = directories
        .iter()
        .find(|(descriptor, _)| *descriptor == event.wd())
    else {
        return false;
    };
    flags.intersects(ReadFlags::DELETE_SELF | ReadFlags::MOVE_SELF)
        || event.file_name().is_some_and(|changed| {
            names
                .iter()
                .any(|name| name.as_bytes() == changed.to_bytes())
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Source;
    use std::fs;

    /// Whether `watch` saw a change since last asked.
    fn changed(watch: &mut Watch) -> bool {
        let now = Instant::now();
        watch.read(now).unwrap();
        watch.settled(now + SETTLE)
    }

    #[test]
    fn saves_in_place_by_renaming_and_by_removing_are_seen() {
        // Given a watched file, each way of saving or removing it is a change, and other files
        // in its directory are not.
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("compust.toml");
        fs::write(&file, "opacity = 90").unwrap();
        let mut watch = Watch::new().unwrap();
        watch.follow(std::slice::from_ref(&file));
        assert!(!changed(&mut watch));

        fs::write(&file, "opacity = 80").unwrap();
        assert!(changed(&mut watch));
        assert!(!changed(&mut watch));

        fs::write(dir.path().join("other.toml"), "opacity = 70").unwrap();
        assert!(!changed(&mut watch));

        let temporary = dir.path().join(".compust.toml.swp");
        fs::write(&temporary, "opacity = 60").unwrap();
        assert!(!changed(&mut watch));
        fs::rename(&temporary, &file).unwrap();
        assert!(changed(&mut watch));

        fs::remove_file(&file).unwrap();
        assert!(changed(&mut watch));
    }

    #[test]
    fn a_change_waits_until_the_files_settle() {
        // Given a save that moves the old file aside and writes a new one, the reload waits
        // for the last step, then comes due once.
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("compust.toml");
        fs::write(&file, "opacity = 90").unwrap();
        let mut watch = Watch::new().unwrap();
        watch.follow(std::slice::from_ref(&file));
        let start = Instant::now();

        fs::rename(&file, dir.path().join("compust.toml~")).unwrap();
        watch.read(start).unwrap();
        assert_eq!(watch.due(), Some(start + SETTLE));
        assert!(!watch.settled(start + SETTLE / 2));

        fs::write(&file, "opacity = 80").unwrap();
        watch.read(start + SETTLE / 2).unwrap();
        assert!(!watch.settled(start + SETTLE));
        assert!(watch.settled(start + SETTLE * 3 / 2));
        assert_eq!(watch.due(), None);
        assert!(!watch.settled(start + SETTLE * 2));
    }

    #[test]
    fn linked_files_are_watched_where_they_point() {
        // Given a configuration that is a link into another directory, as dotfile managers
        // make, editing the file it points to is a change.
        let dir = tempfile::tempdir().unwrap();
        let (home, dotfiles) = (dir.path().join("home"), dir.path().join("dotfiles"));
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&dotfiles).unwrap();
        let target = dotfiles.join("compust.toml");
        fs::write(&target, "opacity = 90").unwrap();
        let link = home.join("compust.toml");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        let mut watch = Watch::new().unwrap();
        watch.follow(&Source::new(Some(link)).files());

        fs::write(&target, "opacity = 80").unwrap();
        assert!(changed(&mut watch));
    }

    #[test]
    fn following_other_files_stops_watching_the_old_ones() {
        let dir = tempfile::tempdir().unwrap();
        let (old, new) = (dir.path().join("old"), dir.path().join("new"));
        fs::create_dir_all(&old).unwrap();
        fs::create_dir_all(&new).unwrap();
        let mut watch = Watch::new().unwrap();
        watch.follow(&[
            old.join("compust.toml"),
            dir.path().join("missing/compust.toml"),
        ]);
        watch.follow(&[new.join("compust.toml")]);

        fs::write(old.join("compust.toml"), "opacity = 80").unwrap();
        assert!(!changed(&mut watch));
        fs::write(new.join("compust.toml"), "opacity = 80").unwrap();
        assert!(changed(&mut watch));
    }
}
