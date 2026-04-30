use std::ffi::c_void;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::collections::HashMap;
use std::sync::Mutex;

type HMODULE = *mut c_void;

extern "system" {
    fn LoadLibraryA(lpLibFileName: *const i8) -> HMODULE;
    fn GetProcAddress(hModule: HMODULE, lpProcName: *const i8) -> *mut c_void;
    fn GetModuleFileNameA(hModule: *mut c_void, lpFilename: *mut u8, nSize: u32) -> u32;
    fn GetLastError() -> u32;
}

static MAIN_DLL_HANDLE: AtomicUsize = AtomicUsize::new(0);
static INIT_ATTEMPTED: AtomicUsize = AtomicUsize::new(0);

// Кэш для функций - значительно ускоряет повторные вызовы
lazy_static::lazy_static! {
    static ref FUNCTION_CACHE: Mutex<HashMap<String, usize>> = Mutex::new(HashMap::new());
    static ref CALL_STATS: Mutex<CallStats> = Mutex::new(CallStats::new());
}

#[derive(Debug)]
struct CallStats {
    total_calls: u64,
    cache_hits: u64,
    cache_misses: u64,
    failed_calls: u64,
    total_time_us: u64,
    min_time_us: u64,
    max_time_us: u64,
}

impl CallStats {
    fn new() -> Self {
        Self {
            total_calls: 0,
            cache_hits: 0,
            cache_misses: 0,
            failed_calls: 0,
            total_time_us: 0,
            min_time_us: u64::MAX,
            max_time_us: 0,
        }
    }

    fn record_call(&mut self, cache_hit: bool, failed: bool, time_us: u64) {
        self.total_calls += 1;
        self.total_time_us += time_us;

        if time_us < self.min_time_us {
            self.min_time_us = time_us;
        }
        if time_us > self.max_time_us {
            self.max_time_us = time_us;
        }

        if failed {
            self.failed_calls += 1;
        } else if cache_hit {
            self.cache_hits += 1;
        } else {
            self.cache_misses += 1;
        }
    }

    fn get_cache_hit_rate(&self) -> f64 {
        if self.total_calls == 0 {
            return 0.0;
        }
        (self.cache_hits as f64 / self.total_calls as f64) * 100.0
    }

    fn get_avg_time_us(&self) -> u64 {
        if self.total_calls == 0 {
            return 0;
        }
        self.total_time_us / self.total_calls
    }
}

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
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let _ = writeln!(f, "[{}] {}", timestamp, msg);
        }
    }
}

fn get_main_dll_handle() -> HMODULE {
    let handle = MAIN_DLL_HANDLE.load(Ordering::Acquire);
    if handle != 0 {
        return handle as HMODULE;
    }

    // Проверяем, не пытались ли мы уже загрузить и не получилось
    if INIT_ATTEMPTED.load(Ordering::Acquire) != 0 {
        return std::ptr::null_mut();
    }

    unsafe {
        log("Loading void_uplay_api.dll...");
        let new_handle = LoadLibraryA(b"void_uplay_api.dll\0".as_ptr() as *const i8);

        if new_handle.is_null() {
            let error = GetLastError();
            log(&format!("ERROR: Failed to load void_uplay_api.dll (error code: {})", error));
            INIT_ATTEMPTED.store(1, Ordering::Release);
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

// Улучшенная функция получения адреса с кэшированием и метриками
unsafe fn get_cached_proc_address(func_name: &str) -> Option<*mut c_void> {
    let start = std::time::Instant::now();
    let mut stats = CALL_STATS.lock().unwrap();

    // Проверяем кэш
    {
        let cache = FUNCTION_CACHE.lock().unwrap();
        if let Some(&addr) = cache.get(func_name) {
            let elapsed = start.elapsed().as_micros() as u64;
            stats.record_call(true, false, elapsed);
            return Some(addr as *mut c_void);
        }
    }

    // Не в кэше - загружаем
    let handle = get_main_dll_handle();
    if handle.is_null() {
        let elapsed = start.elapsed().as_micros() as u64;
        stats.record_call(false, true, elapsed);
        log(&format!("ERROR: Main DLL not loaded for {}", func_name));
        return None;
    }

    let func_name_cstr = format!("{}\0", func_name);
    let proc = GetProcAddress(handle, func_name_cstr.as_ptr() as *const i8);

    if proc.is_null() {
        let elapsed = start.elapsed().as_micros() as u64;
        stats.record_call(false, true, elapsed);
        log(&format!("ERROR: Function {} not found in main DLL", func_name));
        return None;
    }

    // Добавляем в кэш
    {
        let mut cache = FUNCTION_CACHE.lock().unwrap();
        cache.insert(func_name.to_string(), proc as usize);
    }

    let elapsed = start.elapsed().as_micros() as u64;
    stats.record_call(false, false, elapsed);
    Some(proc)
}

// Публичная функция для получения статистики
#[no_mangle]
pub unsafe extern "C" fn VoidProxy_GetStats(buffer: *mut i8, size: u32) -> usize {
    if buffer.is_null() || size == 0 {
        return 0;
    }

    let stats = CALL_STATS.lock().unwrap();
    let cache = FUNCTION_CACHE.lock().unwrap();

    let stats_str = format!(
        "=== Void Proxy Statistics v0.5.0 ===\n\
         Total calls: {}\n\
         Cache hits: {} ({:.2}%)\n\
         Cache misses: {}\n\
         Failed calls: {}\n\
         Cached functions: {}\n\
         \n\
         === Performance ===\n\
         Total time: {:.2}ms\n\
         Avg time: {:.2}μs\n\
         Min time: {}μs\n\
         Max time: {}μs\n\
         \n\
         === Efficiency ===\n\
         Cache speedup: ~{}x\n\
         Calls per second: {:.0}\n",
        stats.total_calls,
        stats.cache_hits,
        stats.get_cache_hit_rate(),
        stats.cache_misses,
        stats.failed_calls,
        cache.len(),
        stats.total_time_us as f64 / 1000.0,
        stats.get_avg_time_us(),
        if stats.min_time_us == u64::MAX { 0 } else { stats.min_time_us },
        stats.max_time_us,
        if stats.cache_misses > 0 { stats.cache_hits / stats.cache_misses.max(1) } else { 0 },
        if stats.total_time_us > 0 { (stats.total_calls as f64 * 1_000_000.0) / stats.total_time_us as f64 } else { 0.0 }
    );

    let bytes = stats_str.as_bytes();
    let copy_len = bytes.len().min((size - 1) as usize);
    std::ptr::copy_nonoverlapping(bytes.as_ptr() as *const i8, buffer, copy_len);
    *(buffer.add(copy_len)) = 0;

    copy_len
}

// Публичная функция для очистки кэша
#[no_mangle]
pub unsafe extern "C" fn VoidProxy_ClearCache() -> usize {
    let mut cache = FUNCTION_CACHE.lock().unwrap();
    let count = cache.len();
    cache.clear();
    log(&format!("Cache cleared: {} entries removed", count));
    count
}

// Публичная функция для получения версии proxy
#[no_mangle]
pub unsafe extern "C" fn VoidProxy_GetVersion() -> u32 {
    0x00050000 // v0.5.0
}

// Публичная функция для сброса статистики
#[no_mangle]
pub unsafe extern "C" fn VoidProxy_ResetStats() -> usize {
    let mut stats = CALL_STATS.lock().unwrap();
    *stats = CallStats::new();
    log("Statistics reset");
    1
}

// Публичная функция для получения размера кэша
#[no_mangle]
pub unsafe extern "C" fn VoidProxy_GetCacheSize() -> usize {
    let cache = FUNCTION_CACHE.lock().unwrap();
    cache.len()
}

// Публичная функция для предварительного кэширования функций
#[no_mangle]
pub unsafe extern "C" fn VoidProxy_PreloadCache(func_names: *const *const i8, count: u32) -> usize {
    if func_names.is_null() || count == 0 {
        return 0;
    }

    let mut loaded = 0;
    for i in 0..count {
        let func_name_ptr = *func_names.offset(i as isize);
        if !func_name_ptr.is_null() {
            if let Ok(func_name) = std::ffi::CStr::from_ptr(func_name_ptr).to_str() {
                if get_cached_proc_address(func_name).is_some() {
                    loaded += 1;
                }
            }
        }
    }

    log(&format!("Preloaded {} functions into cache", loaded));
    loaded
}

// Макрос для создания proxy функций с улучшенной обработкой ошибок
macro_rules! proxy_fn {
    ($name:ident() -> $ret:ty) => {
        #[no_mangle]
        pub unsafe extern "C" fn $name() -> $ret {
            if let Some(proc) = get_cached_proc_address(stringify!($name)) {
                let f: unsafe extern "C" fn() -> $ret = std::mem::transmute(proc);
                f()
            } else {
                0 as $ret
            }
        }
    };

    ($name:ident($($arg:ident: $typ:ty),*) -> $ret:ty) => {
        #[no_mangle]
        pub unsafe extern "C" fn $name($($arg: $typ),*) -> $ret {
            if let Some(proc) = get_cached_proc_address(stringify!($name)) {
                let f: unsafe extern "C" fn($($typ),*) -> $ret = std::mem::transmute(proc);
                f($($arg),*)
            } else {
                0 as $ret
            }
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

proxy_fn!(UPLAY_USER_GetNameUtf8() -> *const i8);
proxy_fn!(UPLAY_USER_GetAccountIdUtf8() -> *const i8);
proxy_fn!(UPLAY_USER_GetUsernameUtf8() -> *const i8);
proxy_fn!(UPLAY_USER_GetEmailUtf8() -> *const i8);
proxy_fn!(UPLAY_USER_IsConnected() -> usize);
proxy_fn!(UPLAY_USER_IsOwned() -> usize);
proxy_fn!(UPLAY_USER_IsInOfflineMode() -> usize);

proxy_fn!(UPLAY_SAVE_GetSavegames(a: *mut c_void, b: *mut c_void) -> usize);
proxy_fn!(UPLAY_SAVE_GetSavegamesResult(a: *mut c_void, b: *mut u32, c: *mut *mut *mut c_void) -> usize);
proxy_fn!(UPLAY_SAVE_Open(a: u32, b: u32, c: *mut u32, d: *mut c_void) -> usize);
proxy_fn!(UPLAY_SAVE_Read(a: u32, b: u32, c: u32, d: *mut c_void, e: *mut usize, f: *mut c_void) -> usize);
proxy_fn!(UPLAY_SAVE_Write(a: u32, b: u32, c: *const c_void, d: *mut c_void) -> usize);
proxy_fn!(UPLAY_SAVE_Remove(a: u32, b: *mut c_void) -> usize);
proxy_fn!(UPLAY_SAVE_Close(a: u32) -> usize);
proxy_fn!(UPLAY_SAVE_ReleaseGameList(a: *mut c_void) -> usize);

proxy_fn!(UPLAY_ACH_EarnAchievement(a: u32, b: *mut c_void) -> usize);
proxy_fn!(UPLAY_ACH_EarnAchivement(a: u32, b: *mut c_void) -> usize);
proxy_fn!(UPLAY_ACH_GetAchievements(a: u32, b: *const i8, c: *mut *mut c_void, d: *mut c_void) -> usize);
proxy_fn!(UPLAY_ACH_Write(a: *const i8) -> usize);

proxy_fn!(UPLAY_OVERLAY_SetShopUrl(a: *const i8, b: *mut c_void) -> usize);
proxy_fn!(UPLAY_OVERLAY_Show(a: u32) -> usize);

proxy_fn!(UPLAY_GetNextEvent(a: *mut isize) -> usize);
proxy_fn!(UPLAY_GetLastError(a: *const i8) -> usize);
proxy_fn!(UPLAY_Release(a: *mut c_void) -> usize);
proxy_fn!(UPLAY_GetVersion() -> usize);
proxy_fn!(UPLAY_GetLocale() -> *const i8);
proxy_fn!(UPLAY_GetTime(a: *mut u64) -> usize);
proxy_fn!(UPLAY_GetOverlayVisibility() -> usize);

proxy_fn!(UPLAY_HasOverlappedOperationCompleted(a: *const c_void) -> usize);
proxy_fn!(UPLAY_GetOverlappedOperationResult(a: *const c_void, b: *mut c_void) -> usize);

proxy_fn!(UPLAY_PRODUCT_IsOwned(a: u32) -> usize);

// ============ UPC API Proxies ============

proxy_fn!(UPC_Init(a: u32, b: i32) -> i32);

#[no_mangle]
pub unsafe extern "C" fn UPC_Uninit() {
    if let Some(proc) = get_cached_proc_address("UPC_Uninit") {
        let f: unsafe extern "C" fn() = std::mem::transmute(proc);
        f()
    }
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

proxy_fn!(UPC_CPUScoreGet(a: *mut c_void, b: *mut u32) -> i32);
proxy_fn!(UPC_GPUScoreGet(a: *mut c_void, b: *mut u32, c: *mut f32) -> i32);

proxy_fn!(UPC_LaunchApp(a: *mut c_void, b: u32, c: *const c_void) -> i32);
proxy_fn!(UPC_FriendListGet(a: *mut c_void, b: u32, c: *mut c_void, d: *const c_void, e: *const c_void) -> i32);

// ============ Additional UPLAY User Proxies ============

proxy_fn!(UPLAY_USER_GetPassword(a: *mut i8, b: u32) -> usize);
proxy_fn!(UPLAY_USER_GetTicket(a: *mut i8, b: u32) -> usize);
proxy_fn!(UPLAY_USER_GetCdKey(a: *mut i8, b: u32, c: *mut c_void) -> usize);
proxy_fn!(UPLAY_USER_GetCredentials(a: *mut i8, b: u32) -> usize);
proxy_fn!(UPLAY_USER_GetConsumableCount(a: u32, b: *mut u32, c: *mut c_void) -> usize);
proxy_fn!(UPLAY_USER_ConsumeConsumable(a: u32, b: u32, c: *mut c_void) -> usize);

// ============ Additional UPLAY Achievement Proxies ============

proxy_fn!(UPLAY_ACH_GetAchievementImage(a: u32, b: *mut c_void, c: u32, d: *mut c_void) -> usize);
proxy_fn!(UPLAY_ACH_ReleaseAchievementList(a: *mut c_void) -> usize);

// ============ Additional UPLAY Save Proxies ============

proxy_fn!(UPLAY_SAVE_SetName(a: u32, b: *const i8) -> usize);
proxy_fn!(UPLAY_SAVE_GetSavegameInfo(a: u32, b: *mut c_void) -> usize);

// ============ Additional UPLAY Overlay Proxies ============

proxy_fn!(UPLAY_OVERLAY_ShowShopUrl(a: *const i8, b: *mut c_void) -> usize);
proxy_fn!(UPLAY_OVERLAY_ShowBrowser(a: *const i8, b: *mut c_void) -> usize);

// ============ Additional UPLAY Product Proxies ============

proxy_fn!(UPLAY_PRODUCT_GetProductList(a: *mut c_void) -> usize);
proxy_fn!(UPLAY_PRODUCT_GetProductListResult(a: *mut c_void, b: *mut *mut c_void) -> usize);
proxy_fn!(UPLAY_PRODUCT_ReleaseProductList(a: *mut c_void) -> usize);

// ============ UPLAY Friends Proxies ============

proxy_fn!(UPLAY_FRIENDS_Init(a: u32) -> usize);
proxy_fn!(UPLAY_FRIENDS_GetFriendList(a: *mut *mut c_void, b: *mut c_void) -> usize);
proxy_fn!(UPLAY_FRIENDS_ReleaseFriendList(a: *mut c_void) -> usize);
proxy_fn!(UPLAY_FRIENDS_RequestFriendship(a: *const i8, b: *mut c_void) -> usize);
proxy_fn!(UPLAY_FRIENDS_IsFriend(a: *const i8) -> usize);
proxy_fn!(UPLAY_FRIENDS_ShowFriendSelectionUI(a: *mut c_void) -> usize);

// ============ UPLAY Avatar Proxies ============

proxy_fn!(UPLAY_AVATAR_GetBitmap(a: *const i8, b: u32, c: *mut c_void, d: *mut c_void) -> usize);
proxy_fn!(UPLAY_AVATAR_GetAvatarIdForCurrentUser(a: *mut u32, b: *mut c_void) -> usize);
proxy_fn!(UPLAY_AVATAR_Release(a: *mut c_void) -> usize);

// ============ UPLAY Party Proxies ============

proxy_fn!(UPLAY_PARTY_Init(a: u32) -> usize);
proxy_fn!(UPLAY_PARTY_GetFullMemberList(a: *mut *mut c_void, b: *mut c_void) -> usize);
proxy_fn!(UPLAY_PARTY_GetInGameMemberList(a: *mut *mut c_void, b: *mut c_void) -> usize);
proxy_fn!(UPLAY_PARTY_ReleaseMemberList(a: *mut c_void) -> usize);
proxy_fn!(UPLAY_PARTY_InvitePartyToGame(a: *const i8, b: *mut c_void) -> usize);
proxy_fn!(UPLAY_PARTY_ShowGameInviteOverlayUI(a: *const i8, b: *mut c_void) -> usize);
proxy_fn!(UPLAY_PARTY_SetUserData(a: *const c_void, b: u32) -> usize);
proxy_fn!(UPLAY_PARTY_GetUserData(a: *const i8, b: *mut c_void, c: *mut u32) -> usize);

// ============ UPLAY Presence Proxies ============

proxy_fn!(UPLAY_PRESENCE_SetPresence(a: u32, b: *const c_void) -> usize);

// ============ UPLAY Metadata Proxies ============

proxy_fn!(UPLAY_METADATA_SetSingleEventTag(a: *const i8, b: *const i8) -> usize);
proxy_fn!(UPLAY_METADATA_SetMultiEventTag(a: *const i8, b: *const i8) -> usize);
proxy_fn!(UPLAY_METADATA_ClearContinuousTag(a: *const i8) -> usize);
proxy_fn!(UPLAY_METADATA_SetContinuousTag(a: *const i8, b: *const i8) -> usize);

// ============ UPLAY Options Proxies ============

proxy_fn!(UPLAY_OPTIONS_Enumerate(a: *const c_void, b: *mut c_void) -> usize);
proxy_fn!(UPLAY_OPTIONS_Get(a: u32, b: *mut c_void, c: u32) -> usize);
proxy_fn!(UPLAY_OPTIONS_Set(a: u32, b: *const c_void, c: u32) -> usize);

// ============ UPLAY Win Proxies ============

proxy_fn!(UPLAY_WIN_GetRewards(a: *mut *mut c_void, b: *mut c_void) -> usize);
proxy_fn!(UPLAY_WIN_ReleaseRewardList(a: *mut c_void) -> usize);
proxy_fn!(UPLAY_WIN_RefreshActions(a: *mut c_void) -> usize);
proxy_fn!(UPLAY_WIN_GetActions(a: *mut *mut c_void, b: *mut c_void) -> usize);
proxy_fn!(UPLAY_WIN_ReleaseActionList(a: *mut c_void) -> usize);

// ============ UPLAY Storage Proxies ============

proxy_fn!(UPLAY_STORAGE_Read(a: *const i8, b: *mut c_void, c: u32, d: *mut u32, e: *mut c_void) -> usize);
proxy_fn!(UPLAY_STORAGE_Write(a: *const i8, b: *const c_void, c: u32, d: *mut c_void) -> usize);
proxy_fn!(UPLAY_STORAGE_Delete(a: *const i8, b: *mut c_void) -> usize);

// ============ UPLAY Monetization Proxies ============

proxy_fn!(UPLAY_MONETIZATION_GetCurrency(a: *mut c_void, b: *mut c_void) -> usize);
proxy_fn!(UPLAY_MONETIZATION_ShowShop(a: *mut c_void) -> usize);

// ============ Additional UPC Proxies ============

proxy_fn!(UPC_UserPlayedWith(a: *mut c_void, b: *const i8) -> i32);
proxy_fn!(UPC_StreamingCountryGet(a: *mut c_void) -> *const i8);
proxy_fn!(UPC_StreamingInputEnable(a: *mut c_void, b: i32) -> i32);
proxy_fn!(UPC_TicketGet(a: *mut c_void, b: *mut i8, c: u32) -> i32);

// ============ DBDATA (Denuvo stub) Proxies ============

proxy_fn!(dbdata_GetVersion() -> u32);
proxy_fn!(dbdata_Init(a: *const c_void) -> i32);
proxy_fn!(dbdata_Uninit() -> i32);
proxy_fn!(dbdata_CheckIntegrity() -> i32);
proxy_fn!(dbdata_Validate(a: *const c_void, b: u32) -> i32);
proxy_fn!(dbdata_GetStatus() -> i32);
proxy_fn!(dbdata_IsActivated() -> i32);
proxy_fn!(dbdata_GetHardwareId(a: *mut i8, b: u32) -> i32);
proxy_fn!(dbdata_GetMachineId(a: *mut i8, b: u32) -> i32);
proxy_fn!(dbdata_Activate(a: *const i8) -> i32);
proxy_fn!(dbdata_Deactivate() -> i32);
proxy_fn!(dbdata_GetActivationCount() -> i32);
proxy_fn!(dbdata_GetActivationLimit() -> i32);
proxy_fn!(dbdata_GetDaysRemaining() -> i32);
proxy_fn!(dbdata_IsTrialVersion() -> i32);
proxy_fn!(dbdata_GetTrialDaysRemaining() -> i32);

#[no_mangle]
pub unsafe extern "system" fn DllMain(_hinst: *const u8, reason: u32, _reserved: *const u8) -> i32 {
    match reason {
        1 => {
            let dll_name = get_source_dll_name();
            log(&format!("=== DllMain: PROCESS_ATTACH ({}) ===", dll_name));
            log("Void Proxy v0.5.0 - Ultimate Edition with advanced caching");
        }
        0 => {
            log("=== DllMain: PROCESS_DETACH ===");
            let stats = CALL_STATS.lock().unwrap();
            let cache = FUNCTION_CACHE.lock().unwrap();

            log(&format!("Final stats: {} total calls, {:.2}% cache hit rate, {} failed",
                stats.total_calls, stats.get_cache_hit_rate(), stats.failed_calls));
            log(&format!("Performance: avg {:.2}μs, min {}μs, max {}μs",
                stats.get_avg_time_us(),
                if stats.min_time_us == u64::MAX { 0 } else { stats.min_time_us },
                stats.max_time_us));
            log(&format!("Cache: {} functions, {:.2}ms total time",
                cache.len(), stats.total_time_us as f64 / 1000.0));
        }
        _ => {}
    }
    1
}
