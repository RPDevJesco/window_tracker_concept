use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime};
use std::thread;
use std::time::Duration as StdDuration;

lazy_static::lazy_static! {
    static ref WINDOWS: Mutex<HashMap<String, f64>> = Mutex::new(HashMap::new());
    static ref LAST_FOCUS_CHANGE: Mutex<SystemTime> = Mutex::new(SystemTime::now());
}

#[cfg(windows)]
mod platform {
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW};

    pub fn get_active_window_title() -> Option<String> {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.is_invalid() {
                return None;
            }

            let mut buffer = [0u16; 512];
            let length = GetWindowTextW(hwnd, &mut buffer);
            if length > 0 {
                Some(String::from_utf16_lossy(&buffer[..length as usize]))
            } else {
                None
            }
        }
    }
}

#[cfg(target_os = "macos")]
mod platform {
    pub fn get_active_window_title() -> Option<String> {
        use core_foundation::base::TCFType;
        let window_list = unsafe { CGWindowListCopyWindowInfo(kCGWindowListOptionOnScreenOnly, 0) };
        if let Some(window_list) = window_list {
            if let Some(window_info) = window_list.get(0) {
                if let Some(window_owner) = window_info.get("kCGWindowOwnerName") {
                    let owner_name: CFString = window_owner.downcast::<CFString>().unwrap();
                    let window_title_str = owner_name.to_string();
                    add_or_update_window(&window_title_str, current_time);
                }
            }
        }
    }
}

#[cfg(target_os = "linux")]
mod platform {
    pub fn get_active_window_title() -> Option<String> {
        let display = unsafe { XOpenDisplay(std::ptr::null()) };
        if !display.is_null() {
            let mut window: u64 = 0;
            let mut revert_to: i32 = 0;
            unsafe { XGetInputFocus(display, &mut window, &mut revert_to) };
            if window != 0 {
                let mut window_name: *mut i8 = std::ptr::null_mut();
                if unsafe { XFetchName(display, window, &mut window_name) } > 0 && !window_name.is_null() {
                    let title = unsafe { CStr::from_ptr(window_name) };
                    let window_title_str = title.to_string_lossy().into_owned();
                    add_or_update_window(&window_title_str, current_time);
                }
                unsafe { XCloseDisplay(display) };
            }
        }
    }
}

use platform::get_active_window_title;

pub fn wt_init() {
    let mut windows = WINDOWS.lock().unwrap();
    windows.clear();
    let mut last_focus_change = LAST_FOCUS_CHANGE.lock().unwrap();
    *last_focus_change = SystemTime::now();
}

pub fn wt_update() {
    let current_time = SystemTime::now();

    if let Some(window_title) = get_active_window_title() {
        add_or_update_window(&window_title, current_time);
    }
}

fn add_or_update_window(title: &str, current_time: SystemTime) {
    let mut windows = WINDOWS.lock().unwrap();
    let mut last_focus_change = LAST_FOCUS_CHANGE.lock().unwrap();
    let elapsed_time = last_focus_change.elapsed().unwrap_or(Duration::from_secs(0)).as_secs_f64();

    if let Some(total_focus_time) = windows.get_mut(title) {
        *total_focus_time += elapsed_time;
    } else {
        windows.insert(title.to_string(), elapsed_time);
    }

    *last_focus_change = current_time;
}

pub fn wt_get_window_count() -> usize {
    let windows = WINDOWS.lock().unwrap();
    windows.len()
}

pub fn wt_get_window_info(index: usize) -> Option<(String, f64)> {
    let windows = WINDOWS.lock().unwrap();
    windows.iter().nth(index).map(|(k, &v)| (k.clone(), v))
}

pub fn wt_get_all_windows() -> Vec<(String, f64)> {
    let windows = WINDOWS.lock().unwrap();
    windows.iter()
        .map(|(k, &v)| (k.clone(), v))
        .collect()
}

pub fn wt_cleanup() {
    let mut windows = WINDOWS.lock().unwrap();
    windows.clear();
}

fn main() {
    wt_init();
    let update_interval = StdDuration::from_millis(100);  // Check active window every 100ms
    let display_interval = StdDuration::from_secs(1);     // Update display every second
    let mut last_display = Instant::now();

    loop {
        wt_update();  // Update window tracking

        // Only display updates every second
        if last_display.elapsed() >= display_interval {
            println!("\nCurrent window tracking status:");
            println!("Number of tracked windows: {}", wt_get_window_count());

            // Display all windows and their times
            for (title, focus_time) in wt_get_all_windows() {
                println!("Window: {}", title);
                println!("  Focus time: {:.1} seconds", focus_time);
            }

            last_display = Instant::now();
        }

        thread::sleep(update_interval);
    }
}