use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use tauri::{AppHandle, LogicalSize, Manager, PhysicalPosition, WebviewWindow, Window};
use tauri_plugin_store::StoreExt;

const STORE: &str = "desktop.json";
const KEY: &str = "main_geometry_v1";
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Geometry {
    schema_version: u8,
    monitor: Option<String>,
    monitor_x: i32,
    monitor_y: i32,
    offset_x: f64,
    offset_y: f64,
    width: f64,
    height: f64,
    maximized: bool,
}
#[derive(Clone, Debug)]
struct Display {
    name: Option<String>,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    scale: f64,
}
#[derive(Default)]
pub struct DesktopState {
    restoring: AtomicBool,
    geometry: Mutex<Option<Geometry>>,
}
#[derive(Debug)]
struct Placement {
    x: i32,
    y: i32,
    width: f64,
    height: f64,
    min_width: f64,
    min_height: f64,
    geometry: Geometry,
}

fn valid(value: &Geometry) -> bool {
    value.schema_version == 1
        && [value.width, value.height, value.offset_x, value.offset_y]
            .iter()
            .all(|number| number.is_finite())
        && (160.0..=10_000.0).contains(&value.width)
        && (160.0..=10_000.0).contains(&value.height)
        && value
            .monitor
            .as_ref()
            .map_or(true, |name| name.len() <= 256)
}
fn place(
    saved: Option<&Geometry>,
    display: &Display,
    border: (u32, u32),
    matching: bool,
) -> Placement {
    let saved = saved.filter(|value| valid(value));
    let scale = if display.scale.is_finite() && display.scale > 0.0 {
        display.scale
    } else {
        1.0
    };
    let available_width = (display.width.saturating_sub(border.0).max(1) as f64) / scale;
    let available_height = (display.height.saturating_sub(border.1).max(1) as f64) / scale;
    let min_width = 960.0_f64.min(available_width);
    let min_height = 640.0_f64.min(available_height);
    let width = saved
        .map_or(1280.0, |value| value.width)
        .clamp(min_width, available_width);
    let height = saved
        .map_or(800.0, |value| value.height)
        .clamp(min_height, available_height);
    let outer_width = width * scale + border.0 as f64;
    let outer_height = height * scale + border.1 as f64;
    let desired_x = saved
        .filter(|_| matching)
        .map_or((display.width as f64 - outer_width) / 2.0, |value| {
            value.offset_x * scale
        });
    let desired_y = saved
        .filter(|_| matching)
        .map_or((display.height as f64 - outer_height) / 2.0, |value| {
            value.offset_y * scale
        });
    let offset_x = desired_x.clamp(0.0, (display.width as f64 - outer_width).max(0.0));
    let offset_y = desired_y.clamp(0.0, (display.height as f64 - outer_height).max(0.0));
    Placement {
        x: (display.x as f64 + offset_x).round() as i32,
        y: (display.y as f64 + offset_y).round() as i32,
        width,
        height,
        min_width,
        min_height,
        geometry: Geometry {
            schema_version: 1,
            monitor: display.name.clone(),
            monitor_x: display.x,
            monitor_y: display.y,
            offset_x: offset_x / scale,
            offset_y: offset_y / scale,
            width,
            height,
            maximized: saved.is_some_and(|value| value.maximized),
        },
    }
}
fn displays(window: &Window) -> Result<Vec<Display>, &'static str> {
    let primary = window.primary_monitor().ok().flatten().map(|monitor| {
        (
            monitor.work_area().position.x,
            monitor.work_area().position.y,
        )
    });
    window
        .available_monitors()
        .map_err(|_| "desktop_monitors_unavailable")
        .map(|monitors| {
            let mut displays: Vec<Display> = monitors
                .into_iter()
                .map(|monitor| {
                    let area = monitor.work_area();
                    Display {
                        name: monitor.name().cloned(),
                        x: area.position.x,
                        y: area.position.y,
                        width: area.size.width,
                        height: area.size.height,
                        scale: monitor.scale_factor(),
                    }
                })
                .collect();
            displays.sort_by_key(|monitor| Some((monitor.x, monitor.y)) != primary);
            displays
        })
}
fn border(window: &Window) -> (u32, u32) {
    match (window.outer_size(), window.inner_size()) {
        (Ok(outer), Ok(inner)) => (
            outer.width.saturating_sub(inner.width),
            outer.height.saturating_sub(inner.height),
        ),
        _ => (0, 0),
    }
}
fn choose<'a>(saved: Option<&Geometry>, monitors: &'a [Display]) -> Option<(&'a Display, bool)> {
    if let Some(value) = saved.filter(|value| valid(value)) {
        if let Some(monitor) = monitors.iter().find(|monitor| {
            value.monitor.is_some() && monitor.name == value.monitor
                || monitor.x == value.monitor_x && monitor.y == value.monitor_y
        }) {
            return Some((monitor, true));
        }
    }
    monitors.first().map(|monitor| (monitor, false))
}
fn persist(app: &AppHandle, geometry: &Geometry, flush: bool) -> Result<(), &'static str> {
    let store = app
        .store(STORE)
        .map_err(|_| "desktop_preferences_unavailable")?;
    store.set(
        KEY,
        serde_json::to_value(geometry).map_err(|_| "desktop_geometry_invalid")?,
    );
    if flush {
        store
            .save()
            .map_err(|_| "desktop_preferences_write_failed")?;
    }
    Ok(())
}
fn apply(window: &Window, saved: Option<&Geometry>) -> Result<Geometry, &'static str> {
    let monitors = displays(window)?;
    let (monitor, matching) = choose(saved, &monitors).ok_or("desktop_monitor_missing")?;
    let placement = place(saved, monitor, border(window), matching);
    window
        .set_min_size(Some(LogicalSize::new(
            placement.min_width,
            placement.min_height,
        )))
        .map_err(|_| "desktop_minimum_failed")?;
    window
        .set_size(LogicalSize::new(placement.width, placement.height))
        .map_err(|_| "desktop_size_failed")?;
    window
        .set_position(PhysicalPosition::new(placement.x, placement.y))
        .map_err(|_| "desktop_position_failed")?;
    if placement.geometry.maximized {
        window.maximize().map_err(|_| "desktop_maximize_failed")?;
    }
    if !matching && saved.is_some() {
        log::warn!("desktop geometry recovered on available monitor");
    }
    Ok(placement.geometry)
}
pub fn restore(app: &AppHandle) {
    let Some(webview) = app.get_webview_window("main") else {
        return;
    };
    let window = webview.as_ref().window();
    let state = app.state::<DesktopState>();
    state.restoring.store(true, Ordering::SeqCst);
    let saved = app
        .store(STORE)
        .ok()
        .and_then(|store| store.get(KEY))
        .and_then(|value| serde_json::from_value::<Geometry>(value).ok())
        .filter(valid);
    match apply(&window, saved.as_ref()) {
        Ok(geometry) => {
            *state
                .geometry
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(geometry);
            log::info!("desktop main geometry restored");
        }
        Err(code) => log::warn!("desktop geometry restore unavailable: {code}"),
    }
    state.restoring.store(false, Ordering::SeqCst);
}
pub fn capture(window: &Window, flush: bool) {
    if window.label() != "main" {
        return;
    }
    let state = window.app_handle().state::<DesktopState>();
    if state.restoring.load(Ordering::SeqCst)
        || window.is_minimized().unwrap_or(true)
        || window.is_fullscreen().unwrap_or(true)
    {
        return;
    }
    let mut current = state
        .geometry
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let maximized = window.is_maximized().unwrap_or(false);
    if !maximized {
        if let (Ok(position), Ok(size), Ok(Some(monitor))) = (
            window.outer_position(),
            window.inner_size(),
            window.current_monitor(),
        ) {
            let area = monitor.work_area();
            let scale = monitor.scale_factor();
            if size.width > 0 && size.height > 0 && scale > 0.0 {
                *current = Some(Geometry {
                    schema_version: 1,
                    monitor: monitor.name().cloned(),
                    monitor_x: area.position.x,
                    monitor_y: area.position.y,
                    offset_x: (position.x as f64 - area.position.x as f64) / scale,
                    offset_y: (position.y as f64 - area.position.y as f64) / scale,
                    width: size.width as f64 / scale,
                    height: size.height as f64 / scale,
                    maximized,
                });
            }
        }
    }
    if let Some(geometry) = current.as_mut() {
        geometry.maximized = maximized;
        if let Err(code) = persist(window.app_handle(), geometry, flush) {
            log::warn!("desktop preferences not saved: {code}");
        }
    }
}

pub fn recover_offscreen(window: &Window) {
    if window.label() != "main"
        || window.is_maximized().unwrap_or(true)
        || window.is_minimized().unwrap_or(true)
        || window.is_fullscreen().unwrap_or(true)
    {
        return;
    }
    let state = window.app_handle().state::<DesktopState>();
    if state.restoring.load(Ordering::SeqCst) {
        return;
    }
    let (Ok(position), Ok(size), Ok(monitors)) = (
        window.outer_position(),
        window.outer_size(),
        displays(window),
    ) else {
        return;
    };
    // Keep a visible title/control strip; normal user panning across monitors is allowed.
    let visible = monitors.iter().any(|monitor| {
        let left = (position.x as i64).max(monitor.x as i64);
        let right =
            (position.x as i64 + size.width as i64).min(monitor.x as i64 + monitor.width as i64);
        let top = (position.y as i64).max(monitor.y as i64);
        let bottom = (position.y as i64 + 48).min(monitor.y as i64 + monitor.height as i64);
        right - left >= 160 && bottom - top >= 24
    });
    if visible {
        return;
    }
    let saved = state
        .geometry
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    state.restoring.store(true, Ordering::SeqCst);
    if let Ok(geometry) = apply(window, saved.as_ref()) {
        *state
            .geometry
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(geometry.clone());
        let _ = persist(window.app_handle(), &geometry, true);
        log::warn!("desktop off-screen main window recovered");
    }
    state.restoring.store(false, Ordering::SeqCst);
}
#[tauri::command]
pub fn desktop_reset_geometry(window: WebviewWindow) -> Result<Geometry, &'static str> {
    if window.label() != "main" {
        return Err("desktop_window_scope_denied");
    }
    let native = window.as_ref().window();
    let state = native.app_handle().state::<DesktopState>();
    state.restoring.store(true, Ordering::SeqCst);
    let result = (|| {
        native
            .set_fullscreen(false)
            .map_err(|_| "desktop_fullscreen_failed")?;
        native
            .unmaximize()
            .map_err(|_| "desktop_unmaximize_failed")?;
        apply(&native, None)
    })();
    state.restoring.store(false, Ordering::SeqCst);
    let geometry = result?;
    *state
        .geometry
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(geometry.clone());
    persist(native.app_handle(), &geometry, true)?;
    log::info!("desktop main geometry reset");
    Ok(geometry)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn monitor(scale: f64) -> Display {
        Display {
            name: Some("primary".into()),
            x: 0,
            y: 0,
            width: 1920,
            height: 1040,
            scale,
        }
    }
    fn saved() -> Geometry {
        Geometry {
            schema_version: 1,
            monitor: Some("removed".into()),
            monitor_x: -1920,
            monitor_y: 0,
            offset_x: 400.0,
            offset_y: 200.0,
            width: 1280.0,
            height: 800.0,
            maximized: false,
        }
    }
    #[test]
    fn dpi_and_removed_monitor_keep_the_whole_window_inside_work_area() {
        for scale in [1.25, 1.5, 2.0] {
            let display = monitor(scale);
            let result = place(Some(&saved()), &display, (16, 8), false);
            assert!(result.x >= 0 && result.y >= 0);
            assert!(result.x as f64 + result.width * scale + 16.0 <= 1921.0);
            assert!(result.y as f64 + result.height * scale + 8.0 <= 1041.0);
        }
    }
    #[test]
    fn malformed_version_and_dimensions_use_default_safe_geometry() {
        let mut input = saved();
        input.width = f64::NAN;
        assert!(!valid(&input));
        let result = place(Some(&input), &monitor(1.0), (0, 0), false);
        assert_eq!(result.width, 1280.0);
        input = saved();
        input.schema_version = 7;
        assert!(!valid(&input));
    }
    #[test]
    fn matching_monitor_restores_logical_size_and_offsets_at_new_dpi() {
        let result = place(Some(&saved()), &monitor(1.25), (0, 0), true);
        assert_eq!(result.width, 1280.0);
        assert_eq!(result.x, 320);
        assert_eq!(result.height, 800.0);
        assert_eq!(result.y, 40);
    }
}
