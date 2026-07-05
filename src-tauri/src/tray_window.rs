const TRAY_WINDOW_VERTICAL_GAP: i32 = 10;
const MAIN_WINDOW_WIDTH: u32 = 760;
const MAIN_WINDOW_HEIGHT: u32 = 560;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PhysicalRect {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) width: i32,
    pub(crate) height: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MonitorBounds {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) width: i32,
    pub(crate) height: i32,
}

pub(crate) fn position_tray_window(
    window: &tauri::WebviewWindow,
    rect: tauri::Rect,
) -> tauri::Result<()> {
    let scale_factor = window.scale_factor()?;
    let tray_rect = physical_rect_from_rect(rect, scale_factor);
    let window_size = window
        .outer_size()
        .unwrap_or_else(|_| tauri::PhysicalSize::new(MAIN_WINDOW_WIDTH, MAIN_WINDOW_HEIGHT));
    let monitor_bounds = resolve_monitor_bounds(window, tray_rect);
    let target = compute_tray_window_position(tray_rect, window_size, monitor_bounds);

    window.set_position(tauri::Position::Physical(target))
}

pub(crate) fn physical_rect_from_rect(rect: tauri::Rect, scale_factor: f64) -> PhysicalRect {
    let (x, y) = match rect.position {
        tauri::Position::Physical(position) => (position.x, position.y),
        tauri::Position::Logical(position) => (
            (position.x * scale_factor).round() as i32,
            (position.y * scale_factor).round() as i32,
        ),
    };
    let (width, height) = match rect.size {
        tauri::Size::Physical(size) => (
            i32::try_from(size.width).unwrap_or(i32::MAX),
            i32::try_from(size.height).unwrap_or(i32::MAX),
        ),
        tauri::Size::Logical(size) => (
            (size.width * scale_factor).round() as i32,
            (size.height * scale_factor).round() as i32,
        ),
    };

    PhysicalRect {
        x,
        y,
        width,
        height,
    }
}

fn resolve_monitor_bounds(
    window: &tauri::WebviewWindow,
    tray_rect: PhysicalRect,
) -> Option<MonitorBounds> {
    let anchor_x = tray_rect.x + (tray_rect.width / 2);
    let anchor_y = tray_rect.y + (tray_rect.height / 2);

    if let Ok(monitors) = window.available_monitors() {
        if let Some(bounds) = monitors
            .into_iter()
            .map(monitor_bounds_from_monitor)
            .find(|bounds| point_within_monitor(*bounds, anchor_x, anchor_y))
        {
            return Some(bounds);
        }
    }

    window
        .current_monitor()
        .ok()
        .flatten()
        .map(monitor_bounds_from_monitor)
}

fn monitor_bounds_from_monitor(monitor: tauri::Monitor) -> MonitorBounds {
    MonitorBounds {
        x: monitor.position().x,
        y: monitor.position().y,
        width: i32::try_from(monitor.size().width).unwrap_or(i32::MAX),
        height: i32::try_from(monitor.size().height).unwrap_or(i32::MAX),
    }
}

fn point_within_monitor(bounds: MonitorBounds, x: i32, y: i32) -> bool {
    let max_x = bounds.x.saturating_add(bounds.width);
    let max_y = bounds.y.saturating_add(bounds.height);
    x >= bounds.x && x < max_x && y >= bounds.y && y < max_y
}

pub(crate) fn compute_tray_window_position(
    tray_rect: PhysicalRect,
    window_size: tauri::PhysicalSize<u32>,
    monitor_bounds: Option<MonitorBounds>,
) -> tauri::PhysicalPosition<i32> {
    let window_width = i32::try_from(window_size.width).unwrap_or(i32::MAX);
    let window_height = i32::try_from(window_size.height).unwrap_or(i32::MAX);
    let centered_x = tray_rect
        .x
        .saturating_add(tray_rect.width / 2)
        .saturating_sub(window_width / 2);
    let below_y = tray_rect
        .y
        .saturating_add(tray_rect.height)
        .saturating_add(TRAY_WINDOW_VERTICAL_GAP);

    if let Some(bounds) = monitor_bounds {
        let max_x = bounds
            .x
            .saturating_add(bounds.width.saturating_sub(window_width).max(0));
        let clamped_x = centered_x.clamp(bounds.x, max_x);

        let max_y = bounds
            .y
            .saturating_add(bounds.height.saturating_sub(window_height).max(0));
        let above_y = tray_rect
            .y
            .saturating_sub(window_height)
            .saturating_sub(TRAY_WINDOW_VERTICAL_GAP);
        let target_y =
            if below_y.saturating_add(window_height) <= bounds.y.saturating_add(bounds.height) {
                below_y
            } else {
                above_y.clamp(bounds.y, max_y)
            };

        return tauri::PhysicalPosition::new(clamped_x, target_y);
    }

    tauri::PhysicalPosition::new(centered_x, below_y)
}
