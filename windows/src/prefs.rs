use std::collections::BTreeMap;
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use windows::Win32::Foundation::WIN32_ERROR;
use windows::Win32::UI::WindowsAndMessaging::{SPI_GETCLIENTAREAANIMATION, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW};
use windows::core::{BOOL, PCWSTR, w};

use snowhush_core::halloween::Choice;
use snowhush_core::modes::{self, Picked};
use snowhush_core::simulation::Mode;

/// Portable build: settings live in `Snowhush.settings` beside the executable (one `name=number` per line)
/// instead of the registry, and nothing is ever written to the registry.
static VALUES: Mutex<Option<BTreeMap<String, u32>>> = Mutex::new(None);

fn settings_file() -> Option<PathBuf> {
    std::env::current_exe().ok().map(|exe| exe.with_file_name("Snowhush.settings"))
}

fn with_values<R>(action: impl FnOnce(&mut BTreeMap<String, u32>) -> R) -> R {
    let mut guard = VALUES.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let values = guard.get_or_insert_with(|| {
        let mut map = BTreeMap::new();
        if let Some(text) = settings_file().and_then(|path| std::fs::read_to_string(path).ok()) {
            for line in text.lines() {
                if let Some((name, value)) = line.split_once('=')
                    && let Ok(value) = value.trim().parse::<u32>()
                {
                    map.insert(name.trim().to_string(), value);
                }
            }
        }
        map
    });
    action(values)
}

#[derive(Clone, Copy)]
pub struct Settings {
    /// How much falls, in percent: 0 to 100 in steps of 5.
    pub intensity: u32,
    /// Snow and leaves settle along the bottom of the screen.
    pub ground: bool,
    pub melt: bool,
    pub snow_on_windows: bool,
    pub behind_windows: bool,
    /// The snow stays over full-screen apps instead of fading out of their way.
    pub over_full_screen: bool,
    /// The snow settles at the bottom of the screen, under the taskbar, for a taskbar made see-through, as by TranslucentTB.
    pub behind_taskbar: bool,
    pub show_in_screenshots: bool,
    /// The snow also settles on what the desktop draws without a window of its own: a selection dragged across it.
    pub deep_interaction: bool,
    pub welcomed: bool,
    /// Unix seconds of the first run, or 0 for an install older than the stamp.
    pub installed_at: u32,
    pub halloween: Choice,
    /// What falls: the kind picked last, or two mixed.
    pub picked: Picked,
    /// Drawn at the display's highest rate all the time, instead of only while something is pushed around.
    pub max_performance: bool,
    /// How opaque what falls is drawn, in percent: 10 to 100 in steps of 5.
    pub opacity: u32,
}

impl Settings {
    /// Whether snow settles anywhere.
    pub fn accumulates(&self) -> bool {
        self.ground || self.snow_on_windows
    }

    pub fn load() -> Settings {
        let calm = !animations_enabled();
        // Before the slider, the intensity was one of Off, Light, Steady, Heavy and Variable.
        let stop = read(w!("snowfall")).map(|i| i.min(4) as usize);
        let welcomed = read(w!("welcomed")).is_some_and(|v| v != 0);
        let installed_at = read(w!("installedAt")).unwrap_or_else(|| {
            let stamp = if welcomed { 0 } else { now() };
            write(w!("installedAt"), stamp);
            stamp
        });
        // Before Accumulate picked where snow settles, it turned settling off everywhere at once.
        let settled = read(w!("accumulate")).is_none_or(|v| v != 0) || read(w!("settleGround")).is_some();
        Settings {
            intensity: read(w!("intensity")).map(|v| v.clamp(0, 100)).or(stop.map(|i| [0, 25, 50, 75, 50][i])).unwrap_or(if calm { 25 } else { 50 }),
            ground: read(w!("settleGround")).map_or(settled, |v| v != 0),
            melt: read(w!("meltSnow")).is_none_or(|v| v != 0),
            snow_on_windows: settled && read(w!("snowOnWindows")).is_none_or(|v| v != 0),
            behind_windows: read(w!("behindWindows")).is_some_and(|v| v != 0),
            over_full_screen: read(w!("showOverFullScreen")).is_some_and(|v| v != 0),
            behind_taskbar: read(w!("behindTaskbar")).is_some_and(|v| v != 0),
            show_in_screenshots: read(w!("showInScreenshots")).is_some_and(|v| v != 0),
            deep_interaction: read(w!("deepInteraction")).is_some_and(|v| v != 0),
            welcomed,
            installed_at,
            halloween: Choice { on: read(w!("halloweenOn")).is_some_and(|v| v != 0), skipped: read(w!("halloweenSkipped")).unwrap_or(0) },
            max_performance: read(w!("performance")).is_some_and(|v| v == 1),
            opacity: read(w!("fallingOpacity")).map_or(100, |v| v.clamp(10, 100)),
            picked: Picked::load(read(w!("mode")), read(w!("autumn")), read(w!("mixed")), [read(w!("mix0")), read(w!("mix1"))]),
        }
    }

    pub fn save(&self) {
        write(w!("intensity"), self.intensity);
        write(w!("settleGround"), self.ground as u32);
        write(w!("meltSnow"), self.melt as u32);
        write(w!("snowOnWindows"), self.snow_on_windows as u32);
        write(w!("behindWindows"), self.behind_windows as u32);
        write(w!("showOverFullScreen"), self.over_full_screen as u32);
        write(w!("behindTaskbar"), self.behind_taskbar as u32);
        write(w!("showInScreenshots"), self.show_in_screenshots as u32);
        write(w!("deepInteraction"), self.deep_interaction as u32);
        write(w!("welcomed"), self.welcomed as u32);
        write(w!("installedAt"), self.installed_at);
        write(w!("halloweenOn"), self.halloween.on as u32);
        write(w!("halloweenSkipped"), self.halloween.skipped);
        write(w!("mode"), modes::number(self.picked.mode));
        write(w!("mixed"), self.picked.mixed as u32);
        write(w!("mix0"), modes::number(self.picked.mix[0]));
        write(w!("mix1"), modes::number(self.picked.mix[1]));
        write(w!("autumn"), (self.picked.mode == Mode::Autumn) as u32);
        write(w!("performance"), self.max_performance as u32);
        write(w!("fallingOpacity"), self.opacity);
    }
}

/// Portable build: there is no "Open at Login", which would write to the registry.
pub fn opens_at_login() -> bool {
    false
}

pub fn set_opens_at_login(_on: bool) -> WIN32_ERROR {
    WIN32_ERROR(1)
}

fn read(name: PCWSTR) -> Option<u32> {
    let name = unsafe { name.to_string() }.ok()?;
    with_values(|values| values.get(&name).copied())
}

fn now() -> u32 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as u32)
}

fn write(name: PCWSTR, value: u32) {
    let Ok(name) = (unsafe { name.to_string() }) else { return };
    with_values(|values| {
        if values.get(&name) == Some(&value) {
            return;
        }
        values.insert(name, value);
        if let Some(path) = settings_file() {
            let text: String = values.iter().map(|(name, value)| format!("{name}={value}\n")).collect();
            let _ = std::fs::write(path, text);
        }
    });
}

/// The "Animation effects" setting: off means the person prefers less motion.
pub fn animations_enabled() -> bool {
    let mut enabled = BOOL(1);
    let result = unsafe {
        SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, Some((&raw mut enabled).cast::<c_void>()), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0))
    };
    result.is_err() || enabled.as_bool()
}
