use std::ffi::c_void;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};

type HMODULE = *mut c_void;

extern "system" {
    fn LoadLibraryA(lpLibFileName: *const i8) -> HMODULE;
    fn GetProcAddress(hModule: HMODULE, lpProcName: *const i8) -> *mut c_void;
    fn GetModuleFileNameA(hModule: *mut c_void, lpFilename: *mut u8, nSize: u32) -> u32;
}

static MAIN_DLL_HANDLE: AtomicUsize = AtomicUsize::new(0);

fn log_path() -> Option<std::path::PathBuf> {
    let mut buf = [0u8; 260];
    unsafe {
        let len = GetModuleFileNameA(std::ptr::null_mut(), buf.as_mut_ptr(), buf.len() as u32);
        if len == 0 {
            return None;
        }
        let s = std::str::from_utf8(&buf[..len as usize]).ok()?;
        let path = std::path::Path::new(s).parent()?.join("void_proxy.log");
        Some(path)
    }
}

fn log(msg: &str) {
    if let Some(path) = log_path() {
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(f, "[PROXY] {}", msg);
        }
    }
}

fn get_main_dll_handle() -> HMODULE {
    let handle = MAIN_DLL_HANDLE.load(Ordering::Acquire);
    if handle != 0 {
        return handle as HMODULE;
    }

    unsafe {
        log("Loading void_uplay_api.dll...");
        let new_handle = LoadLibraryA(b"void_uplay_api.dll\0".as_ptr() as *const i8);
        if new_handle.is_null() {
            log("ERROR: Failed to load void_uplay_api.dll");
            return std::ptr::null_mut();
        }

        log("Successfully loaded void_uplay_api.dll");
        MAIN_DLL_HANDLE.store(new_handle as usize, Ordering::Release);
        new_handle
    }
}

fn get_source_dll_name() -> &'static str {
    unsafe {
        let mut buf = [0u8; 260];
        let len = GetModuleFileNameA(std::ptr::null_mut(), buf.as_mut_ptr(), buf.len() as u32);
        if len > 0 {
            if let Ok(s) = std::str::from_utf8(&buf[..len as usize]) {
                if let Some(filename) = std::path::Path::new(s).file_name() {
                    if let Some(name) = filename.to_str() {
                        return Box::leak(name.to_lowercase().into_boxed_str());
                    }
                }
            }
        }
    }
    "unknown.dll"
}

// Макрос для создания proxy функций
macro_rules! proxy_fn {
    ($name:ident() -> $ret:ty) => {
        #[no_mangle]
        pub unsafe extern "C" fn $name() -> $ret {
            log(&format!("Proxy: {}", stringify!($name)));
            let handle = get_main_dll_handle();
            if handle.is_null() {
                log(&format!("ERROR: Main DLL not loaded for {}", stringify!($name)));
                return 0 as $ret;
            }

            let proc = GetProcAddress(handle, concat!(stringify!($name), "\0").as_ptr() as *const i8);
            if proc.is_null() {
                log(&format!("ERROR: Function {} not found in main DLL", stringify!($name)));
                return 0 as $ret;
            }

            let f: unsafe extern "C" fn() -> $ret = std::mem::transmute(proc);
            f()
        }
    };

    ($name:ident($($arg:ident: $typ:ty),*) -> $ret:ty) => {
        #[no_mangle]
        pub unsafe extern "C" fn $name($($arg: $typ),*) -> $ret {
            log(&format!("Proxy: {}", stringify!($name)));
            let handle = get_main_dll_handle();
            if handle.is_null() {
                log(&format!("ERROR: Main DLL not loaded for {}", stringify!($name)));
                return 0 as $ret;
            }

            let proc = GetProcAddress(handle, concat!(stringify!($name), "\0").as_ptr() as *const i8);
            if proc.is_null() {
                log(&format!("ERROR: Function {} not found in main DLL", stringify!($name)));
                return 0 as $ret;
            }

            let f: unsafe extern "C" fn($($typ),*) -> $ret = std::mem::transmute(proc);
            f($($arg),*)
        }
    };
}

// ============ UPLAY API Proxies ============

proxy_fn!(UPLAY_Init() -> usize);
proxy_fn!(UPLAY_Start(a: u32, b: u32) -> usize);
proxy_fn!(UPLAY_Startup(a: u32, b: u32, c: *const i8) -> usize);
proxy_fn!(UPLAY_Update() -> usize);
proxy_fn!(UPLAY_Quit() -> usize);
proxy_fn!(UPLAY_Qout() -> usize);

proxy_fn!(UPLAY_USER_GetNameUtf8(a: *mut i8, b: u32) -> usize);
proxy_fn!(UPLAY_USER_GetAccountIdUtf8(a: *mut i8, b: u32) -> usize);
proxy_fn!(UPLAY_USER_IsConnected() -> usize);
proxy_fn!(UPLAY_USER_IsOwned() -> usize);
proxy_fn!(UPLAY_USER_IsInOfflineMode() -> usize);

proxy_fn!(UPLAY_SAVE_GetSavegames(a: *mut c_void, b: *mut c_void) -> usize);
proxy_fn!(UPLAY_SAVE_Open(a: u32, b: u32, c: *mut u32, d: *mut c_void) -> usize);
proxy_fn!(UPLAY_SAVE_Read(a: u32, b: u32, c: u32, d: *mut *mut i8, e: *mut usize, f: *mut c_void) -> usize);
proxy_fn!(UPLAY_SAVE_Write(a: u32, b: u32, c: *const *const i8, d: *mut c_void) -> usize);
proxy_fn!(UPLAY_SAVE_Remove(a: u32, b: *mut c_void) -> usize);
proxy_fn!(UPLAY_SAVE_Close(a: u32) -> usize);

proxy_fn!(UPLAY_ACH_EarnAchievement(a: u32, b: *mut c_void) -> usize);
proxy_fn!(UPLAY_ACH_EarnAchivement(a: u32, b: *mut c_void) -> usize);
proxy_fn!(UPLAY_ACH_GetAchievements(a: u32, b: *const i8, c: *mut *mut c_void, d: *mut c_void) -> usize);
proxy_fn!(UPLAY_ACH_Write(a: *const i8) -> usize);

proxy_fn!(UPLAY_OVERLAY_SetShopUrl(a: *const i8, b: *mut c_void) -> usize);
proxy_fn!(UPLAY_OVERLAY_Show(a: u32) -> usize);

proxy_fn!(UPLAY_GetNextEvent(a: *mut isize) -> usize);
proxy_fn!(UPLAY_GetLastError(a: *const i8) -> usize);
proxy_fn!(UPLAY_Release(a: *mut c_void) -> usize);

// ============ UPC API Proxies ============

proxy_fn!(UPC_Init(a: u32, b: i32) -> i32);

#[no_mangle]
pub unsafe extern "C" fn UPC_Uninit() {
    log("Proxy: UPC_Uninit");
    let handle = get_main_dll_handle();
    if handle.is_null() {
        return;
    }
    let proc = GetProcAddress(handle, b"UPC_Uninit\0".as_ptr() as *const i8);
    if proc.is_null() {
        return;
    }
    let f: unsafe extern "C" fn() = std::mem::transmute(proc);
    f()
}

proxy_fn!(UPC_ContextCreate(a: u32, b: *const c_void) -> *mut c_void);
proxy_fn!(UPC_ContextFree(a: *mut c_void) -> i32);
proxy_fn!(UPC_Update(a: *mut c_void) -> i32);
proxy_fn!(UPC_ErrorToString(a: i32) -> *const i8);

proxy_fn!(UPC_EventNextPeek(a: *mut c_void, b: *mut c_void) -> i32);
proxy_fn!(UPC_EventNextPoll(a: *mut c_void, b: *mut c_void) -> i32);

proxy_fn!(UPC_EmailGet(a: *mut c_void) -> *const i8);
proxy_fn!(UPC_IdGet(a: *mut c_void) -> *const i8);
proxy_fn!(UPC_NameGet(a: *mut c_void) -> *const i8);
proxy_fn!(UPC_InstallLanguageGet(a: *mut c_void) -> *const i8);

proxy_fn!(UPC_ProductListGet(a: *mut c_void, b: *const i8, c: u32, d: *mut *mut c_void, e: *const c_void, f: *const c_void) -> i32);
proxy_fn!(UPC_ProductListFree(a: *mut c_void, b: *mut c_void) -> i32);

proxy_fn!(UPC_AchievementUnlock(a: *mut c_void, b: u32, c: *const c_void, d: *const c_void) -> i32);
proxy_fn!(UPC_AchievementListGet(a: *mut c_void, b: *const i8, c: u32, d: *mut c_void, e: *const c_void, f: *const c_void) -> i32);

proxy_fn!(UPC_OverlayShow(a: *mut c_void, b: u32, c: *const c_void, d: *const c_void) -> i32);
proxy_fn!(UPC_OverlayNotificationShow(a: *mut c_void, b: u32) -> i32);

proxy_fn!(UPC_RichPresenceSet(a: *mut c_void, b: u32, c: *const c_void) -> i32);

proxy_fn!(UPC_StorageFileOpen(a: *mut c_void, b: *mut i8, c: u32, d: *mut i32) -> i32);
proxy_fn!(UPC_StorageFileClose(a: *mut c_void, b: i32) -> i32);
proxy_fn!(UPC_StorageFileRead(a: *mut c_void, b: i32, c: i32, d: u32, e: *mut c_void, f: *mut i32, g: *const c_void, h: *const c_void) -> i32);
proxy_fn!(UPC_StorageFileWrite(a: *mut c_void, b: i32, c: *mut c_void, d: i32, e: *const c_void, f: *const c_void) -> i32);

#[no_mangle]
pub unsafe extern "system" fn DllMain(_hinst: *const u8, reason: u32, _reserved: *const u8) -> i32 {
    match reason {
        1 => {
            log(&format!("DllMain: PROCESS_ATTACH ({})", get_source_dll_name()));
        }
        0 => {
            log("DllMain: PROCESS_DETACH");
        }
        _ => {}
    }
    1
}
