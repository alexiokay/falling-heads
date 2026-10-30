//! Local, per-process diagnostics. No network reporting or global Windows changes.
use bevy::{
    log::{
        BoxedLayer,
        tracing_subscriber::{self, fmt::MakeWriter},
    },
    prelude::*,
};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Resource, Clone)]
pub struct Session {
    pub directory: PathBuf,
    writer: LogWriter,
}
#[derive(Clone)]
struct LogWriter(Arc<Mutex<RollingLog>>);
struct RollingLog {
    file: Option<File>,
    directory: PathBuf,
    bytes: u64,
    limit: u64,
}
impl RollingLog {
    fn new(directory: PathBuf, limit: u64) -> io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(directory.join("runtime.log"))?;
        let bytes = file.metadata()?.len();
        Ok(Self {
            file: Some(file),
            directory,
            bytes,
            limit,
        })
    }
    fn append(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.bytes > 0 && self.bytes + bytes.len() as u64 > self.limit {
            self.file.take(); // Close before renaming on Windows.
            let previous = self.directory.join("runtime.previous.log");
            if previous.exists() {
                fs::remove_file(&previous)?;
            }
            fs::rename(self.directory.join("runtime.log"), previous)?;
            self.file = Some(File::create(self.directory.join("runtime.log"))?);
            self.bytes = 0;
        }
        self.file
            .as_mut()
            .ok_or_else(|| io::Error::other("log file unavailable"))?
            .write_all(bytes)?;
        self.bytes += bytes.len() as u64;
        Ok(bytes.len())
    }
}
impl Write for LogWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0
            .lock()
            .map_err(|_| io::Error::other("log lock poisoned"))?
            .append(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        if let Some(file) = self
            .0
            .lock()
            .map_err(|_| io::Error::other("log lock poisoned"))?
            .file
            .as_mut()
        {
            file.flush()?;
        }
        Ok(())
    }
}
impl<'a> MakeWriter<'a> for LogWriter {
    type Writer = Self;
    fn make_writer(&'a self) -> Self {
        self.clone()
    }
}
fn stamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
impl Session {
    pub fn start() -> io::Result<Self> {
        let exe = std::env::current_exe()?;
        let root = exe
            .parent()
            .and_then(|p| p.parent())
            .filter(|p| p.file_name().is_some_and(|n| n == "builds"))
            .and_then(|p| p.parent())
            .map(PathBuf::from)
            .unwrap_or(std::env::current_dir()?);
        let directory = std::env::var_os("MINEHEADS_SESSION_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                root.join("logs")
                    .join(format!("session-{}-{}", stamp(), std::process::id()))
            });
        fs::create_dir_all(&directory)?;
        let session = Self {
            writer: LogWriter(Arc::new(Mutex::new(RollingLog::new(
                directory.clone(),
                8 * 1024 * 1024,
            )?))),
            directory,
        };
        let meta = serde_json::json!({"started_unix_ms":stamp(),"pid":std::process::id(),"executable":exe,
            "version":env!("CARGO_PKG_VERSION"),"working_directory":std::env::current_dir().ok(),
            "asset_root":std::env::var_os("BEVY_ASSET_ROOT"),"status":"running"});
        fs::write(
            session.directory.join("session.json"),
            serde_json::to_vec_pretty(&meta)?,
        )?;
        let panic_path = session.directory.join("panic.txt");
        let old = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            // Independent of the logger mutex: also works if logging itself failed.
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&panic_path)
            {
                let _ = writeln!(
                    file,
                    "Unix ms: {}\nThread: {:?}\n{}\n{}",
                    stamp(),
                    std::thread::current().name(),
                    info,
                    std::backtrace::Backtrace::force_capture()
                );
                let _ = file.sync_all();
            }
            old(info);
        }));
        eprintln!("[DIAGNOSTICS] {}", session.directory.display());
        Ok(session)
    }
    pub fn finish(&self, code: u8) {
        let value = serde_json::json!({"finished_unix_ms":stamp(),"app_exit_code":code});
        let _ = fs::write(
            self.directory.join("app-exit.json"),
            serde_json::to_vec_pretty(&value).unwrap_or_default(),
        );
    }
}
pub fn layer(app: &mut App) -> Option<BoxedLayer> {
    let session = app.world().get_resource::<Session>()?;
    Some(Box::new(
        tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_thread_names(true)
            .with_writer(session.writer.clone()),
    ))
}
pub fn snapshot(
    session: Option<Res<Session>>,
    time: Res<Time<Real>>,
    mut next: Local<f32>,
    settings: Res<crate::resources::graphics_settings::GraphicsSettings>,
    seed: Res<crate::resources::world_seed::WorldSeed>,
    weather: Res<crate::components::weather::ActiveWeather>,
    state: Res<State<crate::states::GameState>>,
    hero: Query<&Transform, With<crate::components::hero::Hero>>,
    actors: Query<
        (),
        Or<(
            With<crate::components::villager::Villager>,
            With<crate::components::enemy::Enemy>,
        )>,
    >,
    blocks: Query<(), With<crate::components::block::Block>>,
) {
    let Some(session) = session else {
        return;
    };
    if time.elapsed_secs() < *next {
        return;
    }
    *next = time.elapsed_secs() + 10.;
    let value = serde_json::json!({"unix_ms":stamp(),"elapsed_seconds":time.elapsed_secs(),"seed":seed.seed,
        "state":format!("{:?}",state.get()),"graphics":&*settings,"weather":format!("{:?}",weather.weather_type),
        "hero":hero.iter().next().map(|t|t.translation.to_array()),"other_heads":actors.iter().count(),"blocks":blocks.iter().count()});
    let _ = fs::write(
        session.directory.join("last-state.json"),
        serde_json::to_vec_pretty(&value).unwrap_or_default(),
    );
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn log_rollover_keeps_recent_evidence_and_closes_windows_handles() {
        let directory = std::env::temp_dir().join(format!(
            "mineheads-log-test-{}-{}",
            std::process::id(),
            stamp()
        ));
        fs::create_dir(&directory).unwrap();
        let mut log = RollingLog::new(directory.clone(), 10).unwrap();
        log.append(b"first\n").unwrap();
        log.append(b"second\n").unwrap();
        log.append(b"third\n").unwrap();
        assert_eq!(
            fs::read(directory.join("runtime.previous.log")).unwrap(),
            b"second\n"
        );
        assert_eq!(fs::read(directory.join("runtime.log")).unwrap(), b"third\n");
        drop(log);
        fs::remove_file(directory.join("runtime.previous.log")).unwrap();
        fs::remove_file(directory.join("runtime.log")).unwrap();
        fs::remove_dir(directory).unwrap();
    }
}
