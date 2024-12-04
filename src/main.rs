#![windows_subsystem = "windows"]

use std::sync::Mutex;
use std::fs::OpenOptions;
use std::io::Write;
use windows::Win32::Foundation::*;
use windows::Win32::System::Power::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::SystemServices::{GUID_MONITOR_POWER_ON, GUID_LIDSWITCH_STATE_CHANGE};
use windows::Win32::System::Shutdown::LockWorkStation;
use windows::Win32::System::Threading::CreateMutexW;

const APP_NAME: &str = "lidlock";
const SINGLETON_IDENTIFIER: &str = "Global\\{3DA16D16-5F02-4CFD-8C43-11C31127889D}";
const TIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

struct Logger {
    file: Option<Mutex<std::fs::File>>,
}

impl Logger {
    fn new(path: Option<&str>) -> Self {
        let file = path.and_then(|p| {
            OpenOptions::new()
                .write(true)
                .create(true)
                .append(true)
                .open(p)
                .ok()
                .map(|f| Mutex::new(f))
        });
        Logger { file }
    }

    fn log(&self, message: &str) {
        if let Some(file) = &self.file {
            if let Ok(mut file_guard) = file.lock() {
                let now = chrono::Local::now();
                let timestamp = now.format(TIME_FORMAT);
                let log_line = format!("[{}] {}\n", timestamp, message);
                let _ = file_guard.write_all(log_line.as_bytes());
                let _ = file_guard.flush();
            }
        }
    }
}

struct LidLockWindow {
    hwnd: HWND,
    logger: Logger,
}

impl LidLockWindow {
    fn new(logger: Logger) -> windows::core::Result<Self> {
        logger.log("Creating LidLockWindow");
        
        unsafe {
            let instance = GetModuleHandleW(None)?;
            
            logger.log("Registering window class");
            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(Self::window_proc),
                hInstance: instance,
                lpszClassName: windows::core::PCWSTR(wide_string(APP_NAME).as_ptr()),
                ..Default::default()
            };

            if RegisterClassExW(&wc) == 0 {
                return Err(windows::core::Error::from_win32());
            }

            logger.log("Creating window");
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                windows::core::PCWSTR(wide_string(APP_NAME).as_ptr()),
                None,
                WINDOW_STYLE(0),
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                HWND_MESSAGE,
                None,
                instance,
                None,
            );

            if hwnd.0 == 0 {
                return Err(windows::core::Error::from_win32());
            }

            let window = LidLockWindow { hwnd, logger };
            window.register_notifications()?;

            Ok(window)
        }
    }

    fn register_notifications(&self) -> windows::core::Result<()> {
        unsafe {
            self.logger.log("Registering power notifications");
            
            let handle = HANDLE(self.hwnd.0);
            
            if RegisterPowerSettingNotification(
                handle,
                &GUID_MONITOR_POWER_ON,
                DEVICE_NOTIFY_WINDOW_HANDLE.0 as u32,
            ).is_err() {
                self.logger.log("Failed to register GUID_MONITOR_POWER_ON notification");
                return Err(windows::core::Error::from_win32());
            }

            if RegisterPowerSettingNotification(
                handle,
                &GUID_LIDSWITCH_STATE_CHANGE,
                DEVICE_NOTIFY_WINDOW_HANDLE.0 as u32,
            ).is_err() {
                self.logger.log("Failed to register GUID_LIDSWITCH_STATE_CHANGE notification");
                return Err(windows::core::Error::from_win32());
            }

            Ok(())
        }
    }

    fn run(&self) -> windows::core::Result<()> {
        self.logger.log("Starting message loop");
        
        unsafe {
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, HWND(0), 0, 0).as_bool() {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            Ok(())
        }
    }

    unsafe extern "system" fn window_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        let logger = Logger::new(None);

        match msg {
            WM_POWERBROADCAST => {
                logger.log("Received WM_POWERBROADCAST");
                
                if wparam.0 == PBT_POWERSETTINGCHANGE as usize {
                    logger.log("Received PBT_POWERSETTINGCHANGE");
                    
                    let setting = &*(lparam.0 as *const POWERBROADCAST_SETTING);
                    let state = *(setting.Data.as_ptr() as *const u32);
                    
                    logger.log(&format!("Power setting state: {}", state));

                    if state == 0 {
                        if GetSystemMetrics(SM_REMOTESESSION) == 0 {
                            logger.log("Attempting to lock workstation");
                            
                            if LockWorkStation().as_bool() {
                                logger.log("Workstation locked successfully");
                            } else {
                                logger.log("Failed to lock workstation");
                            }
                        } else {
                            logger.log("Ignoring, session is remote");
                        }
                    } else {
                        logger.log("Ignoring non-zero state");
                    }
                }
            }
            _ => return DefWindowProcW(hwnd, msg, wparam, lparam),
        }
        LRESULT(0)
    }
}

pub struct SingletonHandle {
    _mutex: Mutex<()>,
}

impl SingletonHandle {
    pub fn new() -> windows::core::Result<Self> {
        unsafe {
            let _mutex = CreateMutexW(
                None,
                false,
                windows::core::PCWSTR(wide_string(SINGLETON_IDENTIFIER).as_ptr()),
            )?;

            if GetLastError() == ERROR_ALREADY_EXISTS {
                return Err(windows::core::Error::new(
                    windows::core::HRESULT(0x800700B7u32 as i32),
                    "Application instance already exists".into(),
                ));
            }

            Ok(SingletonHandle {
                _mutex: Mutex::new(()),
            })
        }
    }
}

fn wide_string(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn main() -> windows::core::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    
    // Determine log path based on arguments
    let log_path = if args.iter().any(|arg| arg == "--debug") {
        // Get temp dir and append our log file name
        let temp_path = std::env::temp_dir().join("lidlock.log");
        temp_path.to_string_lossy().into_owned()
    } else {
        // If a specific path was provided as first arg, use that
        args.get(1)
            .filter(|arg| *arg != "--debug")
            .map(|s| s.to_string())
            .unwrap_or_default()
    };

    let logger = Logger::new(if log_path.is_empty() { None } else { Some(log_path.as_str()) });
    logger.log("Main started");

    let _singleton = SingletonHandle::new()?;

    let window = LidLockWindow::new(logger)?;
    window.run()
}