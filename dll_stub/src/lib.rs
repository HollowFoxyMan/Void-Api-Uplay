use std::ffi::c_void;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::sync::Mutex;
use std::collections::HashMap;
use std::path::PathBuf;

#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;

static LOG_MUTEX: Mutex<()> = Mutex::new(());

fn log(msg: &str) {
    let _guard = LOG_MUTEX.lock().unwrap();
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open("void_uplay_api.log") {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let _ = writeln!(f, "[{}] {}", timestamp, msg);
    }
}

// Расширенная конфигурация
#[derive(Debug, Clone)]
struct Config {
    username: String,
    account_id: String,
    email: String,
    language: String,
    game_id: String,
    game_name: String,
    profile_name: String,
    dlc_unlock_all: bool,
    auto_detect_game: bool,
    detailed_logging: bool,
    performance_mode: bool,
    cache_enabled: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            username: "VoidUser".to_string(),
            account_id: "5d3e2202-106b-46fc-b71f-000e5e593556".to_string(),
            email: "voiduplay@api".to_string(),
            language: "en-US".to_string(),
            game_id: "4311".to_string(),
            game_name: "Unknown".to_string(),
            profile_name: "VoidUser".to_string(),
            dlc_unlock_all: true,
            auto_detect_game: true,
            detailed_logging: false,
            performance_mode: true,
            cache_enabled: true,
        }
    }
}

impl Config {
    fn load() -> Self {
        let mut config = Config::default();

        // Пробуем найти INI файл
        let ini_paths = vec![
            PathBuf::from("void_uplay_api.ini"),
            get_exe_dir().join("void_uplay_api.ini"),
        ];

        for path in ini_paths {
            if path.exists() {
                log(&format!("Loading config from: {:?}", path));
                if let Ok(content) = fs::read_to_string(&path) {
                    // Простой парсинг INI
                    for line in content.lines() {
                        let line = line.trim();
                        if line.is_empty() || line.starts_with('[') || line.starts_with(';') || line.starts_with('#') {
                            continue;
                        }

                        if let Some((key, value)) = line.split_once('=') {
                            let key = key.trim();
                            let value = value.trim();

                            match key {
                                "UserName" => config.username = value.to_string(),
                                "AccountId" => config.account_id = value.to_string(),
                                "Email" => config.email = value.to_string(),
                                "Language" => config.language = value.to_string(),
                                "GameName" => config.game_name = value.to_string(),
                                "GameId" => config.game_id = value.to_string(),
                                "ProfileName" => config.profile_name = value.to_string(),
                                "DLCUnlockall" => config.dlc_unlock_all = value.to_lowercase() == "true",
                                "AutoDetectGame" => config.auto_detect_game = value.to_lowercase() == "true",
                                "DetailedLogging" => config.detailed_logging = value.to_lowercase() == "true",
                                "PerformanceMode" => config.performance_mode = value.to_lowercase() == "true",
                                "CacheEnabled" => config.cache_enabled = value.to_lowercase() == "true",
                                _ => {}
                            }
                        }
                    }

                    log(&format!("Config loaded: user={}, game={}", config.username, config.game_name));
                    break;
                }
            }
        }

        // Автоопределение игры если включено
        if config.auto_detect_game && config.game_name == "Unknown" {
            config.game_name = detect_game_name();
            log(&format!("Auto-detected game: {}", config.game_name));
        }

        config
    }
}

// Автоопределение игры по имени процесса
fn detect_game_name() -> String {
    unsafe {
        let mut buf = [0u8; 260];

        extern "system" {
            fn GetModuleFileNameA(hModule: *mut std::ffi::c_void, lpFilename: *mut u8, nSize: u32) -> u32;
        }

        let len = GetModuleFileNameA(std::ptr::null_mut(), buf.as_mut_ptr(), buf.len() as u32);
        if len > 0 {
            if let Ok(path) = std::str::from_utf8(&buf[..len as usize]) {
                if let Some(filename) = std::path::Path::new(path).file_stem() {
                    if let Some(name) = filename.to_str() {
                        let name_lower = name.to_lowercase();

                        // База данных известных игр
                        if name_lower.contains("farcry5") || name_lower.contains("fc5") {
                            return "Far Cry 5".to_string();
                        } else if name_lower.contains("farcry4") || name_lower.contains("fc4") {
                            return "Far Cry 4".to_string();
                        } else if name_lower.contains("farcry3") || name_lower.contains("fc3") {
                            return "Far Cry 3".to_string();
                        } else if name_lower.contains("assassin") {
                            if name_lower.contains("origins") {
                                return "Assassin's Creed Origins".to_string();
                            } else if name_lower.contains("odyssey") {
                                return "Assassin's Creed Odyssey".to_string();
                            } else if name_lower.contains("valhalla") {
                                return "Assassin's Creed Valhalla".to_string();
                            }
                            return "Assassin's Creed".to_string();
                        } else if name_lower.contains("watchdogs") || name_lower.contains("watch_dogs") {
                            if name_lower.contains("2") {
                                return "Watch Dogs 2".to_string();
                            }
                            return "Watch Dogs".to_string();
                        } else if name_lower.contains("division") {
                            if name_lower.contains("2") {
                                return "The Division 2".to_string();
                            }
                            return "The Division".to_string();
                        } else if name_lower.contains("rainbow") || name_lower.contains("r6") {
                            return "Rainbow Six Siege".to_string();
                        } else if name_lower.contains("ghost") {
                            if name_lower.contains("wildlands") {
                                return "Ghost Recon Wildlands".to_string();
                            } else if name_lower.contains("breakpoint") {
                                return "Ghost Recon Breakpoint".to_string();
                            }
                            return "Ghost Recon".to_string();
                        } else if name_lower.contains("crew") {
                            return "The Crew".to_string();
                        } else if name_lower.contains("steep") {
                            return "Steep".to_string();
                        }

                        return name.to_string();
                    }
                }
            }
        }
    }
    "Unknown".to_string()
}

fn get_exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

// Глобальная конфигурация
lazy_static::lazy_static! {
    static ref CONFIG: Mutex<Config> = Mutex::new(Config::load());
    static ref EVENT_QUEUE: Mutex<Vec<UplayEvent>> = Mutex::new(Vec::new());
}

// Структура события Uplay (правильная)
#[repr(C)]
#[derive(Clone, Copy)]
struct UplayEvent {
    event_type: u32,
    dummy: u32,
    overlapped: usize,
}

impl UplayEvent {
    fn new(event_type: u32) -> Self {
        Self {
            event_type,
            dummy: 0,
            overlapped: 0,
        }
    }
}

// Улучшенная система полок с метриками
#[derive(Debug)]
struct ApiShelf {
    name: &'static str,
    api_type: ApiType,
    call_count: u64,
    last_call: Option<String>,
    error_count: u64,
    total_time_us: u64,
    min_time_us: u64,
    max_time_us: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ApiType {
    Uplay,
    Upc,
}

impl ApiShelf {
    fn new(name: &'static str, api_type: ApiType) -> Self {
        Self {
            name,
            api_type,
            call_count: 0,
            last_call: None,
            error_count: 0,
            total_time_us: 0,
            min_time_us: u64::MAX,
            max_time_us: 0,
        }
    }

    fn record_call(&mut self, func_name: &str) {
        self.call_count += 1;
        self.last_call = Some(func_name.to_string());

        let config = CONFIG.lock().unwrap();
        if config.detailed_logging {
            log(&format!("[{}] Call #{}: {}", self.name, self.call_count, func_name));
        }
    }

    fn record_call_with_time(&mut self, func_name: &str, time_us: u64) {
        self.record_call(func_name);
        self.total_time_us += time_us;
        if time_us < self.min_time_us {
            self.min_time_us = time_us;
        }
        if time_us > self.max_time_us {
            self.max_time_us = time_us;
        }
    }

    fn record_error(&mut self, func_name: &str) {
        self.error_count += 1;
        log(&format!("[{}] ERROR in {}", self.name, func_name));
    }

    fn get_stats(&self) -> String {
        let avg_time = if self.call_count > 0 {
            self.total_time_us / self.call_count
        } else {
            0
        };

        format!(
            "Shelf '{}': {} calls, {} errors, last: {:?}, avg: {}μs, min: {}μs, max: {}μs",
            self.name,
            self.call_count,
            self.error_count,
            self.last_call,
            avg_time,
            if self.min_time_us == u64::MAX { 0 } else { self.min_time_us },
            self.max_time_us
        )
    }

    fn get_performance_stats(&self) -> String {
        let avg_time = if self.call_count > 0 {
            self.total_time_us / self.call_count
        } else {
            0
        };

        format!(
            "{}: {:.2}ms total, {:.2}μs avg",
            self.name,
            self.total_time_us as f64 / 1000.0,
            avg_time
        )
    }
}

// Глобальное хранилище полок с улучшенной организацией
lazy_static::lazy_static! {
    static ref SHELVES: Mutex<HashMap<String, ApiShelf>> = {
        let mut shelves = HashMap::new();

        // UPLAY полки
        shelves.insert("uplay_r1.dll".to_string(), ApiShelf::new("UPLAY_R1", ApiType::Uplay));
        shelves.insert("uplay_r1_loader.dll".to_string(), ApiShelf::new("UPLAY_R1_LOADER", ApiType::Uplay));
        shelves.insert("uplay_r1_loader64.dll".to_string(), ApiShelf::new("UPLAY_R1_LOADER64", ApiType::Uplay));
        shelves.insert("uplay_r2_loader.dll".to_string(), ApiShelf::new("UPLAY_R2_LOADER", ApiType::Uplay));
        shelves.insert("uplay_r2_loader64.dll".to_string(), ApiShelf::new("UPLAY_R2_LOADER64", ApiType::Uplay));

        // UPC полки
        shelves.insert("upc_r1_loader.dll".to_string(), ApiShelf::new("UPC_R1_LOADER", ApiType::Upc));
        shelves.insert("upc_r1_loader64.dll".to_string(), ApiShelf::new("UPC_R1_LOADER64", ApiType::Upc));
        shelves.insert("upc_r2_loader.dll".to_string(), ApiShelf::new("UPC_R2_LOADER", ApiType::Upc));
        shelves.insert("upc_r2_loader64.dll".to_string(), ApiShelf::new("UPC_R2_LOADER64", ApiType::Upc));

        // Дополнительные
        shelves.insert("dbdata.dll".to_string(), ApiShelf::new("DBDATA", ApiType::Uplay));
        shelves.insert("dbdata_x64.dll".to_string(), ApiShelf::new("DBDATA_X64", ApiType::Uplay));

        Mutex::new(shelves)
    };
}

// Публичная функция для получения статистики
#[no_mangle]
pub unsafe extern "C" fn VoidAPI_GetStats(buffer: *mut i8, size: u32) -> usize {
    if buffer.is_null() || size == 0 {
        return 0;
    }

    let shelves = SHELVES.lock().unwrap();
    let config = CONFIG.lock().unwrap();
    let mut stats = String::from("=== Void API Statistics v0.5.0 ===\n");
    stats.push_str(&format!("Game: {}\n", config.game_name));
    stats.push_str(&format!("User: {}\n", config.username));
    stats.push_str(&format!("Performance Mode: {}\n", config.performance_mode));
    stats.push_str(&format!("Cache Enabled: {}\n\n", config.cache_enabled));

    for (dll_name, shelf) in shelves.iter() {
        stats.push_str(&format!("{}: {}\n", dll_name, shelf.get_stats()));
    }

    let bytes = stats.as_bytes();
    let copy_len = bytes.len().min((size - 1) as usize);
    std::ptr::copy_nonoverlapping(bytes.as_ptr() as *const i8, buffer, copy_len);
    *(buffer.add(copy_len)) = 0; // null terminator

    copy_len
}

// Публичная функция для получения метрик производительности
#[no_mangle]
pub unsafe extern "C" fn VoidAPI_GetPerformanceStats(buffer: *mut i8, size: u32) -> usize {
    if buffer.is_null() || size == 0 {
        return 0;
    }

    let shelves = SHELVES.lock().unwrap();
    let mut stats = String::from("=== Performance Statistics ===\n");

    for (dll_name, shelf) in shelves.iter() {
        if shelf.call_count > 0 {
            stats.push_str(&format!("{}: {}\n", dll_name, shelf.get_performance_stats()));
        }
    }

    let bytes = stats.as_bytes();
    let copy_len = bytes.len().min((size - 1) as usize);
    std::ptr::copy_nonoverlapping(bytes.as_ptr() as *const i8, buffer, copy_len);
    *(buffer.add(copy_len)) = 0;

    copy_len
}

// Публичная функция для перезагрузки конфигурации
#[no_mangle]
pub unsafe extern "C" fn VoidAPI_ReloadConfig() -> usize {
    log("Reloading configuration...");
    let new_config = Config::load();
    *CONFIG.lock().unwrap() = new_config;
    log("Configuration reloaded");
    1
}

// Публичная функция для включения/выключения детального логирования
#[no_mangle]
pub unsafe extern "C" fn VoidAPI_SetDetailedLogging(enabled: i32) -> usize {
    let mut config = CONFIG.lock().unwrap();
    config.detailed_logging = enabled != 0;
    log(&format!("Detailed logging: {}", config.detailed_logging));
    1
}

// Публичная функция для получения версии API
#[no_mangle]
pub unsafe extern "C" fn VoidAPI_GetVersion() -> u32 {
    0x00050000 // v0.5.0
}

// Публичная функция для получения информации о игре
#[no_mangle]
pub unsafe extern "C" fn VoidAPI_GetGameInfo(buffer: *mut i8, size: u32) -> usize {
    if buffer.is_null() || size == 0 {
        return 0;
    }

    let config = CONFIG.lock().unwrap();
    let info = format!("Game: {}\nID: {}\nUser: {}\nLanguage: {}\n",
        config.game_name, config.game_id, config.username, config.language);

    let bytes = info.as_bytes();
    let copy_len = bytes.len().min((size - 1) as usize);
    std::ptr::copy_nonoverlapping(bytes.as_ptr() as *const i8, buffer, copy_len);
    *(buffer.add(copy_len)) = 0;

    copy_len
}

// Публичная функция для сброса статистики
#[no_mangle]
pub unsafe extern "C" fn VoidAPI_ResetStats() -> usize {
    let mut shelves = SHELVES.lock().unwrap();
    for (_, shelf) in shelves.iter_mut() {
        shelf.call_count = 0;
        shelf.error_count = 0;
        shelf.total_time_us = 0;
        shelf.min_time_us = u64::MAX;
        shelf.max_time_us = 0;
        shelf.last_call = None;
    }
    log("Statistics reset");
    1
}

// Улучшенная функция маршрутизации
#[no_mangle]
pub unsafe extern "C" fn VoidAPI_RouteCall(
    source_dll: *const i8,
    function_name: *const i8,
    arg1: *const c_void,
    arg2: *const c_void,
) -> usize {
    if source_dll.is_null() || function_name.is_null() {
        log("RouteCall: NULL parameters");
        return 0;
    }

    let source = std::ffi::CStr::from_ptr(source_dll).to_string_lossy();
    let func = std::ffi::CStr::from_ptr(function_name).to_string_lossy();

    let mut shelves = SHELVES.lock().unwrap();

    if let Some(shelf) = shelves.get_mut(source.as_ref()) {
        shelf.record_call(&func);

        // Вызываем соответствующую функцию
        match func.as_ref() {
            // UPLAY Core
            "UPLAY_Init" => uplay_init(),
            "UPLAY_Start" => uplay_start(arg1 as u32, arg2 as u32),
            "UPLAY_Startup" => uplay_startup(arg1 as u32, arg2 as u32, arg1 as *const i8),
            "UPLAY_Update" => 1,
            "UPLAY_Quit" | "UPLAY_Qout" => 1,
            "UPLAY_Release" => 1,
            "UPLAY_GetNextEvent" => 0,
            "UPLAY_GetLastError" => 0,
            "UPLAY_GetVersion" => 100,
            "UPLAY_GetLocale" => uplay_get_locale() as usize,
            "UPLAY_GetTime" => uplay_get_time(arg1 as *mut u64),
            "UPLAY_HasOverlappedOperationCompleted" => uplay_has_overlapped_completed(arg1 as *const c_void),
            "UPLAY_GetOverlappedOperationResult" => uplay_get_overlapped_result(arg1 as *const c_void, arg2 as *mut c_void),

            // UPLAY User
            "UPLAY_USER_GetNameUtf8" | "UPLAY_USER_GetUsernameUtf8" => uplay_user_get_name_utf8() as usize,
            "UPLAY_USER_GetAccountIdUtf8" => uplay_user_get_account_id_utf8() as usize,
            "UPLAY_USER_GetEmailUtf8" => uplay_user_get_email_utf8() as usize,
            "UPLAY_USER_IsConnected" | "UPLAY_USER_IsOwned" | "UPLAY_USER_IsInOfflineMode" => 1,

            // UPLAY Achievements
            "UPLAY_ACH_EarnAchievement" | "UPLAY_ACH_EarnAchivement" => uplay_ach_earn(arg1 as u32, arg2 as *mut c_void),

            // UPLAY Save
            "UPLAY_SAVE_GetSavegames" => uplay_save_get_savegames(arg1 as *mut *mut c_void, arg2 as *mut c_void),
            "UPLAY_SAVE_GetSavegamesResult" => uplay_save_get_savegames_result(arg1 as *mut c_void, arg2 as *mut u32, arg1 as *mut *mut *mut c_void),
            "UPLAY_SAVE_Open" => uplay_save_open(arg1 as u32, arg2 as u32, arg1 as *mut u32, arg2 as *mut c_void),
            "UPLAY_SAVE_Close" => 1,
            "UPLAY_SAVE_Remove" => uplay_save_remove(arg1 as u32, arg2 as *mut c_void),
            "UPLAY_SAVE_ReleaseGameList" => 1,
            "UPLAY_SAVE_SetName" => 1,
            "UPLAY_SAVE_GetLocalPath" => 1,
            "UPLAY_SAVE_GetCloudPath" => 1,
            "UPLAY_SAVE_Read" => 0,
            "UPLAY_SAVE_Write" => 0,

            // UPLAY Overlay
            "UPLAY_OVERLAY_SetShopUrl" => uplay_overlay_set_shop_url(arg1 as *const i8, arg2 as *mut c_void),
            "UPLAY_OVERLAY_Show" => 1,
            "UPLAY_GetOverlayVisibility" => 0,

            // UPLAY Products
            "UPLAY_PRODUCT_IsOwned" => 1,
            "UPLAY_PRODUCT_GetProductList" => uplay_product_get_product_list(arg1 as *mut c_void),
            "UPLAY_PRODUCT_GetProductListResult" => uplay_product_get_product_list_result(arg1 as *mut c_void, arg2 as *mut *mut c_void),
            "UPLAY_PRODUCT_ReleaseProductList" => uplay_product_release_product_list(arg1 as *mut c_void),

            // UPC Core
            "UPC_Init" => upc_init(arg1 as u32, arg2 as i32),
            "UPC_Uninit" => { upc_uninit(); 0 },
            "UPC_ContextCreate" => upc_context_create(arg1 as u32, arg2) as usize,
            "UPC_ContextFree" => upc_context_free(arg1 as *mut c_void),
            "UPC_Update" => upc_update(arg1 as *mut c_void),

            // UPC User Info
            "UPC_EmailGet" => upc_email_get(arg1 as *mut c_void) as usize,
            "UPC_IdGet" => upc_id_get(arg1 as *mut c_void) as usize,
            "UPC_NameGet" => upc_name_get(arg1 as *mut c_void) as usize,
            "UPC_InstallLanguageGet" => upc_language_get(arg1 as *mut c_void) as usize,

            _ => {
                log(&format!("Unknown function: {}", func));
                0
            }
        }
    } else {
        log(&format!("No shelf found for: {}", source));
        0
    }
}

fn uplay_product_get_product_list(overlapped: *mut c_void) -> usize {
    unsafe { UPLAY_PRODUCT_GetProductList(overlapped) }
}

fn uplay_product_get_product_list_result(overlapped: *mut c_void, list: *mut *mut c_void) -> usize {
    unsafe { UPLAY_PRODUCT_GetProductListResult(overlapped, list) }
}

fn uplay_product_release_product_list(list: *mut c_void) -> usize {
    unsafe { UPLAY_PRODUCT_ReleaseProductList(list) }
}

// ============ UPLAY API Implementation ============

fn uplay_init() -> usize {
    log("UPLAY_Init");
    let config = CONFIG.lock().unwrap();
    log(&format!("Initialized for user: {}", config.username));
    1
}

fn uplay_start(id: u32, flags: u32) -> usize {
    log(&format!("UPLAY_Start(id={}, flags={})", id, flags));

    // Инициализируем API если еще не инициализирован
    uplay_init();

    let config = CONFIG.lock().unwrap();
    log(&format!("Returning success (0) for game: {}", config.game_name));

    // Добавляем событие инициализации в очередь
    let mut queue = EVENT_QUEUE.lock().unwrap();
    queue.push(UplayEvent::new(0)); // Event type 0 = Init complete
    log("Added init event to queue");

    0 // UPLAY_OK = 0 (not 1!)
}

fn uplay_startup(id: u32, ver: u32, _lang: *const i8) -> usize {
    log(&format!("UPLAY_Startup(id={}, ver={})", id, ver));
    1
}

fn get_leaked_str(s: &str) -> *const i8 {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    Box::leak(v.into_boxed_slice()).as_ptr() as *const i8
}

fn uplay_user_get_name_utf8() -> *const i8 {
    let config = CONFIG.lock().unwrap();
    log(&format!("UPLAY_USER_GetNameUtf8 -> {}", config.username));
    get_leaked_str(&config.username)
}

fn uplay_user_get_account_id_utf8() -> *const i8 {
    let config = CONFIG.lock().unwrap();
    log(&format!("UPLAY_USER_GetAccountIdUtf8 -> {}", config.account_id));
    get_leaked_str(&config.account_id)
}

fn uplay_user_get_email_utf8() -> *const i8 {
    let config = CONFIG.lock().unwrap();
    log(&format!("UPLAY_USER_GetEmailUtf8 -> {}", config.email));
    get_leaked_str(&config.email)
}

fn uplay_get_locale() -> *const i8 {
    let config = CONFIG.lock().unwrap();
    log(&format!("UPLAY_GetLocale -> {}", config.language));
    get_leaked_str(&config.language)
}

fn uplay_get_time(time_ptr: *mut u64) -> usize {
    if !time_ptr.is_null() {
        unsafe {
            *time_ptr = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
        }
    }
    1
}

#[repr(C)]
pub struct UplayOverlapped {
    pub reserved: *mut c_void,
    pub is_completed: u32,
    pub error_code: i32,
}

impl UplayOverlapped {
    pub fn set_completed(&mut self) {
        self.is_completed = 1;
        self.error_code = 0;
    }
}

fn uplay_has_overlapped_completed(overlapped: *const c_void) -> usize {
    if !overlapped.is_null() {
        unsafe {
            let ovl = overlapped as *const UplayOverlapped;
            if (*ovl).is_completed == 1 {
                return 1;
            }
        }
    }
    0
}

fn uplay_get_overlapped_result(overlapped: *const c_void, result: *mut c_void) -> usize {
    if !overlapped.is_null() && !result.is_null() {
        unsafe {
            let ovl = overlapped as *const UplayOverlapped;
            if (*ovl).is_completed == 1 {
                *(result as *mut i32) = (*ovl).error_code;
                return 1;
            }
        }
    }
    0
}

fn uplay_ach_earn(achievement_id: u32, overlapped: *mut c_void) -> usize {
    log(&format!("UPLAY_ACH_EarnAchievement(id={})", achievement_id));
    if !overlapped.is_null() {
        unsafe {
            let ovl = overlapped as *mut UplayOverlapped;
            (*ovl).set_completed();
        }
    }
    1
}

fn uplay_save_get_savegames(list: *mut *mut c_void, overlapped: *mut c_void) -> usize {
    log(&format!("UPLAY_SAVE_GetSavegames (R1) - list={:?}, overlapped={:?}", list, overlapped));
    if !overlapped.is_null() {
        unsafe {
            let ovl = overlapped as *mut UplayOverlapped;
            (*ovl).set_completed();
        }
    }
    log("UPLAY_SAVE_GetSavegames -> Returning 0");
    0
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_SAVE_GetSavegamesResult(_overlapped: *mut c_void, count: *mut u32, list: *mut *mut *mut c_void) -> usize {
    log("UPLAY_SAVE_GetSavegamesResult (Enhanced)");
    if !count.is_null() {
        *count = 0;
    }
    if !list.is_null() {
        // Ожидается *mut *mut *mut c_void, т.е. указатель на указатель на массив указателей.
        // Чтобы вернуть "пустой список", достаточно записать NULL в *list (указатель на массив).
        *list = std::ptr::null_mut();
    }
    1
}

fn uplay_save_get_savegames_result(overlapped: *mut c_void, count: *mut u32, list: *mut *mut *mut c_void) -> usize {
    unsafe { UPLAY_SAVE_GetSavegamesResult(overlapped, count, list) }
}

fn uplay_save_open(slot_id: u32, _mode: u32, save_handle: *mut u32, overlapped: *mut c_void) -> usize {
    log(&format!("UPLAY_SAVE_Open(slot={})", slot_id));
    if !save_handle.is_null() {
        unsafe {
            *save_handle = slot_id;
        }
    }
    if !overlapped.is_null() {
        unsafe {
            let ovl = overlapped as *mut UplayOverlapped;
            (*ovl).set_completed();
        }
    }
    1
}

fn uplay_save_remove(slot_id: u32, overlapped: *mut c_void) -> usize {
    log(&format!("UPLAY_SAVE_Remove(slot={})", slot_id));
    if !overlapped.is_null() {
        unsafe {
            let ovl = overlapped as *mut UplayOverlapped;
            (*ovl).set_completed();
        }
    }
    1
}

fn uplay_overlay_set_shop_url(_url_utf8: *const i8, overlapped: *mut c_void) -> usize {
    log("UPLAY_OVERLAY_SetShopUrl");
    if !overlapped.is_null() {
        unsafe {
            let ovl = overlapped as *mut UplayOverlapped;
            (*ovl).set_completed();
        }
    }
    1
}

// ============ UPC API Implementation ============

#[repr(C)]
pub struct UPC_Context {
    pub version: u32,
    pub email: *const i8,
    pub name: *const i8,
    pub id: *const i8,
    pub language: *const i8,
    pub appid: u32,
}

fn upc_init(version: u32, appid: i32) -> usize {
    log(&format!("UPC_Init(version={}, appid={})", version, appid));
    0
}

fn upc_uninit() {
    log("UPC_Uninit");
}

fn upc_context_create(version: u32, _settings: *const c_void) -> *mut UPC_Context {
    log(&format!("UPC_ContextCreate(version={})", version));
    let config = CONFIG.lock().unwrap();

    let ctx = Box::new(UPC_Context {
        version: 0x20220811,
        email: Box::leak(format!("{}\0", config.email).into_boxed_str()).as_ptr() as *const i8,
        name: Box::leak(format!("{}\0", config.username).into_boxed_str()).as_ptr() as *const i8,
        id: Box::leak(format!("{}\0", config.account_id).into_boxed_str()).as_ptr() as *const i8,
        language: Box::leak(format!("{}\0", config.language).into_boxed_str()).as_ptr() as *const i8,
        appid: 0,
    });
    Box::into_raw(ctx)
}

fn upc_context_free(context: *mut c_void) -> usize {
    log("UPC_ContextFree");
    if !context.is_null() {
        unsafe {
            let _ = Box::from_raw(context as *mut UPC_Context);
        }
    }
    0
}

fn upc_update(_context: *mut c_void) -> usize {
    0
}

fn upc_email_get(context: *mut c_void) -> *const i8 {
    if context.is_null() {
        return b"\0".as_ptr() as *const i8;
    }
    unsafe { (*(context as *mut UPC_Context)).email }
}

fn upc_id_get(context: *mut c_void) -> *const i8 {
    if context.is_null() {
        return b"\0".as_ptr() as *const i8;
    }
    unsafe { (*(context as *mut UPC_Context)).id }
}

fn upc_name_get(context: *mut c_void) -> *const i8 {
    if context.is_null() {
        return b"\0".as_ptr() as *const i8;
    }
    unsafe { (*(context as *mut UPC_Context)).name }
}

fn upc_language_get(context: *mut c_void) -> *const i8 {
    if context.is_null() {
        return b"en-US\0".as_ptr() as *const i8;
    }
    unsafe { (*(context as *mut UPC_Context)).language }
}

// ============ Direct Export Functions ============

#[no_mangle]
pub unsafe extern "C" fn UPLAY_Init() -> usize {
    uplay_init()
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_Start(id: u32, flags: u32) -> usize {
    uplay_start(id, flags)
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_Startup(id: u32, ver: u32, lang: *const i8) -> usize {
    uplay_startup(id, ver, lang)
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_Update() -> usize {
    1
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_Quit() -> usize {
    1
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_USER_GetNameUtf8() -> *const i8 {
    uplay_user_get_name_utf8()
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_USER_GetAccountIdUtf8() -> *const i8 {
    uplay_user_get_account_id_utf8()
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_USER_IsConnected() -> usize {
    1
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_USER_IsOwned() -> usize {
    1
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_USER_IsInOfflineMode() -> usize {
    1
}

#[no_mangle]
pub unsafe extern "C" fn UPC_Init(version: u32, appid: i32) -> i32 {
    upc_init(version, appid) as i32
}

#[no_mangle]
pub unsafe extern "C" fn UPC_Uninit() {
    upc_uninit();
}

#[no_mangle]
pub unsafe extern "C" fn UPC_ContextCreate(version: u32, settings: *const c_void) -> *mut UPC_Context {
    upc_context_create(version, settings)
}

#[no_mangle]
pub unsafe extern "C" fn UPC_ContextFree(context: *mut UPC_Context) -> i32 {
    upc_context_free(context as *mut c_void) as i32
}

#[no_mangle]
pub unsafe extern "C" fn UPC_Update(context: *mut UPC_Context) -> i32 {
    upc_update(context as *mut c_void) as i32
}

#[no_mangle]
pub unsafe extern "system" fn DllMain(_hinst: *const u8, reason: u32, _reserved: *const u8) -> i32 {
    match reason {
        1 => {
            log("=== DllMain: PROCESS_ATTACH ===");
            log(&format!("Void Uplay API v0.5.0 - Ultimate Edition"));
            let config = CONFIG.lock().unwrap();
            log(&format!("User: {}, Game: {}", config.username, config.game_name));
            log(&format!("Performance Mode: {}, Cache: {}, Detailed Logging: {}",
                config.performance_mode, config.cache_enabled, config.detailed_logging));
            log(&format!("DLC Unlock All: {}, Auto Detect: {}",
                config.dlc_unlock_all, config.auto_detect_game));
        },
        0 => {
            log("=== DllMain: PROCESS_DETACH ===");
            let shelves = SHELVES.lock().unwrap();
            let mut total_calls = 0u64;
            let mut total_errors = 0u64;

            for (dll, shelf) in shelves.iter() {
                total_calls += shelf.call_count;
                total_errors += shelf.error_count;
                if shelf.call_count > 0 {
                    log(&format!("Final stats - {}: {}", dll, shelf.get_stats()));
                }
            }

            log(&format!("=== Total: {} calls, {} errors ===", total_calls, total_errors));
        },
        _ => {}
    }
    1
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_SAVE_ReleaseGameList(_list: *mut c_void) -> usize {
    log("UPLAY_SAVE_ReleaseGameList");
    1
}

// ============ Additional UPLAY Core Functions ============

#[no_mangle]
pub unsafe extern "C" fn UPLAY_Qout() -> usize { 1 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_Release(_handle: *mut c_void) -> usize { 1 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_GetNextEvent(event: *mut c_void) -> usize {
    if event.is_null() {
        return 0;
    }

    let mut queue = EVENT_QUEUE.lock().unwrap();
    if let Some(ev) = queue.pop() {
        log(&format!("UPLAY_GetNextEvent: returning event type {}", ev.event_type));
        std::ptr::copy_nonoverlapping(&ev as *const UplayEvent as *const u8, event as *mut u8, std::mem::size_of::<UplayEvent>());
        return 1;
    }

    0
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_GetLastError(_msg: *const i8) -> usize { 0 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_GetVersion() -> usize { 100 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_GetLocale() -> *const i8 {
    uplay_get_locale()
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_GetTime(time_ptr: *mut u64) -> usize {
    uplay_get_time(time_ptr)
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_GetOverlayVisibility() -> usize { 0 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_HasOverlappedOperationCompleted(overlapped: *const c_void) -> usize {
    uplay_has_overlapped_completed(overlapped)
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_GetOverlappedOperationResult(overlapped: *const c_void, result: *mut c_void) -> usize {
    uplay_get_overlapped_result(overlapped, result)
}

// ============ UPLAY User Functions ============

#[no_mangle]
pub unsafe extern "C" fn UPLAY_USER_GetUsernameUtf8() -> *const i8 {
    uplay_user_get_name_utf8()
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_USER_GetEmailUtf8() -> *const i8 {
    uplay_user_get_email_utf8()
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_USER_GetPassword(_buffer: *mut i8, _size: u32) -> usize { 0 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_USER_GetTicket(buffer: *mut i8, size: u32) -> usize {
    log("UPLAY_USER_GetTicket");
    if !buffer.is_null() && size > 0 {
        let ticket = b"VOID_TICKET_DUMMY_DATA_LONG_ENOUGH_TO_BE_VALID\0";
        let copy_len = ticket.len().min(size as usize);
        std::ptr::copy_nonoverlapping(ticket.as_ptr() as *const i8, buffer, copy_len);
        if copy_len < size as usize {
            *buffer.add(copy_len) = 0;
        }
    }
    1
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_USER_GetCdKey(buffer: *mut i8, size: u32, overlapped: *mut c_void) -> usize {
    log("UPLAY_USER_GetCdKey");
    if !buffer.is_null() && size > 0 {
        let key = b"VOID-VOID-VOID-VOID-VOID\0";
        let copy_len = key.len().min(size as usize);
        std::ptr::copy_nonoverlapping(key.as_ptr() as *const i8, buffer, copy_len);
        if copy_len < size as usize {
            *buffer.add(copy_len) = 0;
        }
    }
    if !overlapped.is_null() {
        let ovl = overlapped as *mut UplayOverlapped;
        (*ovl).set_completed();
    }
    1
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_USER_GetCredentials(_buffer: *mut i8, _size: u32) -> usize { 0 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_USER_GetConsumableCount(_id: u32, _count: *mut u32, _overlapped: *mut c_void) -> usize { 0 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_USER_ConsumeConsumable(_id: u32, _count: u32, _overlapped: *mut c_void) -> usize { 0 }

// ============ UPLAY Achievement Functions ============

#[no_mangle]
pub unsafe extern "C" fn UPLAY_ACH_EarnAchievement(achievement_id: u32, overlapped: *mut c_void) -> usize {
    uplay_ach_earn(achievement_id, overlapped)
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_ACH_EarnAchivement(achievement_id: u32, overlapped: *mut c_void) -> usize {
    uplay_ach_earn(achievement_id, overlapped)
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_ACH_GetAchievements(_id: u32, _filter: *const i8, _list: *mut *mut c_void, _overlapped: *mut c_void) -> usize { 0 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_ACH_Write(_data: *const i8) -> usize { 1 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_ACH_GetAchievementImage(_id: u32, _buffer: *mut c_void, _size: u32, _overlapped: *mut c_void) -> usize { 0 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_ACH_ReleaseAchievementList(_list: *mut c_void) -> usize { 1 }

// ============ UPLAY Save Functions ============

fn get_save_dir() -> PathBuf {
    let config = CONFIG.lock().unwrap();
    let mut path = if let Ok(local_appdata) = std::env::var("LOCALAPPDATA") {
        PathBuf::from(local_appdata)
    } else {
        get_exe_dir()
    };
    
    path.push("Void Api");
    path.push(&config.game_id);
    
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    path
}

fn get_save_path(slot_id: u32) -> PathBuf {
    get_save_dir().join(format!("save_{}.bin", slot_id))
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_SAVE_GetSavegames(list: *mut *mut c_void, overlapped: *mut c_void) -> usize {
    uplay_save_get_savegames(list, overlapped)
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_SAVE_Open(slot_id: u32, mode: u32, save_handle: *mut u32, overlapped: *mut c_void) -> usize {
    log(&format!("UPLAY_SAVE_Open(slot={}, mode={})", slot_id, mode));
    
    if !save_handle.is_null() {
        *save_handle = slot_id;
    }

    if !overlapped.is_null() {
        let ovl = overlapped as *mut UplayOverlapped;
        (*ovl).set_completed();
    }
    0 // Success
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_SAVE_Close(_handle: u32) -> usize {
    log(&format!("UPLAY_SAVE_Close(handle={})", _handle));
    0 // Success
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_SAVE_Read(handle: u32, size: u32, offset: u32, buffer: *mut c_void, bytes_read: *mut usize, overlapped: *mut c_void) -> usize {
    log(&format!("UPLAY_SAVE_Read(handle={}, size={}, offset={})", handle, size, offset));
    
    let path = get_save_path(handle);
    let mut read = 0;

    if path.exists() {
        if let Ok(data) = fs::read(&path) {
            let data_len = data.len();
            if offset < data_len as u32 {
                let to_read = size.min(data_len as u32 - offset) as usize;
                std::ptr::copy_nonoverlapping(data[offset as usize..].as_ptr() as *const c_void, buffer, to_read);
                read = to_read;
            }
        }
    }

    if !bytes_read.is_null() {
        *bytes_read = read;
    }

    if !overlapped.is_null() {
        let ovl = overlapped as *mut UplayOverlapped;
        (*ovl).set_completed();
    }
    
    if read > 0 { 1 } else { 0 }
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_SAVE_Write(handle: u32, size: u32, buffer: *const c_void, overlapped: *mut c_void) -> usize {
    log(&format!("UPLAY_SAVE_Write(handle={}, size={})", handle, size));
    
    let path = get_save_path(handle);
    let data = std::slice::from_raw_parts(buffer as *const u8, size as usize);
    
    let _ = fs::write(path, data);

    if !overlapped.is_null() {
        let ovl = overlapped as *mut UplayOverlapped;
        (*ovl).set_completed();
    }
    1
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_SAVE_Remove(slot_id: u32, overlapped: *mut c_void) -> usize {
    log(&format!("UPLAY_SAVE_Remove(slot={})", slot_id));
    let path = get_save_path(slot_id);
    let _ = fs::remove_file(path);

    if !overlapped.is_null() {
        let ovl = overlapped as *mut UplayOverlapped;
        (*ovl).set_completed();
    }
    1
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_SAVE_SetName(_handle: u32, _name: *const i8) -> usize { 1 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_SAVE_GetSavegameInfo(_handle: u32, _info: *mut c_void) -> usize { 0 }

// ============ UPLAY Overlay Functions ============

#[no_mangle]
pub unsafe extern "C" fn UPLAY_OVERLAY_Show(_section: u32) -> usize { 1 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_OVERLAY_SetShopUrl(url: *const i8, overlapped: *mut c_void) -> usize {
    uplay_overlay_set_shop_url(url, overlapped)
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_OVERLAY_ShowShopUrl(_url: *const i8, _overlapped: *mut c_void) -> usize { 1 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_OVERLAY_ShowBrowser(_url: *const i8, _overlapped: *mut c_void) -> usize { 1 }

// ============ UPLAY Product Functions ============

#[repr(C)]
pub struct UplayProductList {
    pub count: u32,
    pub ids: *mut u32,
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_PRODUCT_IsOwned(product_id: u32) -> usize {
    let config = CONFIG.lock().unwrap();
    if config.dlc_unlock_all {
        log(&format!("UPLAY_PRODUCT_IsOwned(id={}) -> 1 (Spoofed)", product_id));
        1
    } else {
        log(&format!("UPLAY_PRODUCT_IsOwned(id={}) -> 1 (Default)", product_id));
        1
    }
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_PRODUCT_GetProductList(overlapped: *mut c_void) -> usize {
    log("UPLAY_PRODUCT_GetProductList (R1)");
    
    if !overlapped.is_null() {
        let ovl = overlapped as *mut UplayOverlapped;
        (*ovl).set_completed();
    }
    log("UPLAY_PRODUCT_GetProductList -> Returning 0");
    0
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_PRODUCT_GetProductListResult(_overlapped: *mut c_void, list: *mut *mut c_void) -> usize {
    log("UPLAY_PRODUCT_GetProductListResult");
    let config = CONFIG.lock().unwrap();

    if !list.is_null() && config.dlc_unlock_all {
        let count = 50; // Уменьшим до 50 для надежности
        let mut ids = Vec::with_capacity(count);
        for i in 0..count {
            ids.push(i as u32 + 1);
        }

        let product_list = Box::new(UplayProductList {
            count: count as u32,
            ids: Box::into_raw(ids.into_boxed_slice()) as *mut u32,
        });

        *list = Box::into_raw(product_list) as *mut c_void;
        log(&format!("Returned {} spoofed product IDs", count));
    }
    1
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_PRODUCT_ReleaseProductList(list: *mut c_void) -> usize {
    log("UPLAY_PRODUCT_ReleaseProductList");
    if !list.is_null() {
        let product_list = Box::from_raw(list as *mut UplayProductList);
        if !product_list.ids.is_null() {
            let _ = Box::from_raw(std::slice::from_raw_parts_mut(product_list.ids, product_list.count as usize));
        }
    }
    1
}

// ============ UPLAY Friends Functions ============

#[no_mangle]
pub unsafe extern "C" fn UPLAY_FRIENDS_Init(_flags: u32) -> usize { 1 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_FRIENDS_GetFriendList(_list: *mut *mut c_void, _overlapped: *mut c_void) -> usize { 0 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_FRIENDS_ReleaseFriendList(_list: *mut c_void) -> usize { 1 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_FRIENDS_RequestFriendship(_account_id: *const i8, _overlapped: *mut c_void) -> usize { 0 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_FRIENDS_IsFriend(_account_id: *const i8) -> usize { 0 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_FRIENDS_ShowFriendSelectionUI(_overlapped: *mut c_void) -> usize { 0 }

// ============ UPLAY Avatar Functions ============

#[no_mangle]
pub unsafe extern "C" fn UPLAY_AVATAR_GetBitmap(_account_id: *const i8, _size: u32, _bitmap: *mut c_void, _overlapped: *mut c_void) -> usize { 0 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_AVATAR_GetAvatarIdForCurrentUser(_avatar_id: *mut u32, _overlapped: *mut c_void) -> usize { 0 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_AVATAR_Release(_bitmap: *mut c_void) -> usize { 1 }

// ============ UPLAY Party Functions ============

#[no_mangle]
pub unsafe extern "C" fn UPLAY_PARTY_Init(_flags: u32) -> usize { 1 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_PARTY_GetFullMemberList(_list: *mut *mut c_void, _overlapped: *mut c_void) -> usize { 0 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_PARTY_GetInGameMemberList(_list: *mut *mut c_void, _overlapped: *mut c_void) -> usize { 0 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_PARTY_ReleaseMemberList(_list: *mut c_void) -> usize { 1 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_PARTY_InvitePartyToGame(_game_session_id: *const i8, _overlapped: *mut c_void) -> usize { 0 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_PARTY_ShowGameInviteOverlayUI(_game_session_id: *const i8, _overlapped: *mut c_void) -> usize { 0 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_PARTY_SetUserData(_data: *const c_void, _size: u32) -> usize { 1 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_PARTY_GetUserData(_account_id: *const i8, _data: *mut c_void, _size: *mut u32) -> usize { 0 }

// ============ UPLAY Presence Functions ============

#[no_mangle]
pub unsafe extern "C" fn UPLAY_PRESENCE_SetPresence(_presence_id: u32, _token_list: *const c_void) -> usize { 1 }

// ============ UPLAY Metadata Functions ============

#[no_mangle]
pub unsafe extern "C" fn UPLAY_METADATA_SetSingleEventTag(_tag: *const i8, _value: *const i8) -> usize { 1 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_METADATA_SetMultiEventTag(_tag: *const i8, _value: *const i8) -> usize { 1 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_METADATA_ClearContinuousTag(_tag: *const i8) -> usize { 1 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_METADATA_SetContinuousTag(_tag: *const i8, _value: *const i8) -> usize { 1 }

// ============ UPLAY Options Functions ============

#[no_mangle]
pub unsafe extern "C" fn UPLAY_OPTIONS_Enumerate(_callback: *const c_void, _user_data: *mut c_void) -> usize { 0 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_OPTIONS_Get(_option_id: u32, _buffer: *mut c_void, _size: u32) -> usize { 0 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_OPTIONS_Set(_option_id: u32, _buffer: *const c_void, _size: u32) -> usize { 0 }

// ============ UPLAY Win Functions ============

#[no_mangle]
pub unsafe extern "C" fn UPLAY_WIN_GetRewards(_list: *mut *mut c_void, _overlapped: *mut c_void) -> usize { 0 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_WIN_ReleaseRewardList(_list: *mut c_void) -> usize { 1 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_WIN_RefreshActions(_overlapped: *mut c_void) -> usize { 0 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_WIN_GetActions(_list: *mut *mut c_void, _overlapped: *mut c_void) -> usize { 0 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_WIN_ReleaseActionList(_list: *mut c_void) -> usize { 1 }

// ============ UPLAY Storage Functions ============

#[no_mangle]
pub unsafe extern "C" fn UPLAY_STORAGE_Read(_filename: *const i8, _buffer: *mut c_void, _size: u32, _bytes_read: *mut u32, _overlapped: *mut c_void) -> usize { 0 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_STORAGE_Write(_filename: *const i8, _buffer: *const c_void, _size: u32, _overlapped: *mut c_void) -> usize { 0 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_STORAGE_Delete(_filename: *const i8, _overlapped: *mut c_void) -> usize { 0 }

// ============ UPLAY Monetization Functions ============

#[no_mangle]
pub unsafe extern "C" fn UPLAY_MONETIZATION_GetCurrency(_currency: *mut c_void, _overlapped: *mut c_void) -> usize { 0 }

#[no_mangle]
pub unsafe extern "C" fn UPLAY_MONETIZATION_ShowShop(_overlapped: *mut c_void) -> usize { 0 }

// ============ UPC Extended Functions ============

#[no_mangle]
pub unsafe extern "C" fn UPC_ErrorToString(_error: i32) -> *const i8 {
    b"No error\0".as_ptr() as *const i8
}

#[no_mangle]
pub unsafe extern "C" fn UPC_EventNextPeek(_context: *mut c_void, _event: *mut c_void) -> i32 { 0 }

#[no_mangle]
pub unsafe extern "C" fn UPC_EventNextPoll(_context: *mut c_void, _event: *mut c_void) -> i32 { 0 }

#[no_mangle]
pub unsafe extern "C" fn UPC_EmailGet(context: *mut c_void) -> *const i8 {
    upc_email_get(context)
}

#[no_mangle]
pub unsafe extern "C" fn UPC_IdGet(context: *mut c_void) -> *const i8 {
    upc_id_get(context)
}

#[no_mangle]
pub unsafe extern "C" fn UPC_NameGet(context: *mut c_void) -> *const i8 {
    upc_name_get(context)
}

#[no_mangle]
pub unsafe extern "C" fn UPC_InstallLanguageGet(context: *mut c_void) -> *const i8 {
    upc_language_get(context)
}

#[repr(C)]
pub struct UPC_Product {
    pub appid: u32,
    pub product_type: u32, // 1: Game, 2: DLC, 3: Consumable
    pub ownership_type: u32, // 1: Owned, 2: Subscription, 3: Trial
    pub state: u32, // 1: Installed
}

#[repr(C)]
pub struct UPC_ProductList {
    pub count: u32,
    pub products: *mut UPC_Product,
}

#[no_mangle]
pub unsafe extern "C" fn UPC_ProductListGet(_context: *mut c_void, _filter: *const i8, _flags: u32, list: *mut *mut c_void, _callback: *const c_void, _user_data: *const c_void) -> i32 {
    log("UPC_ProductListGet");
    let config = CONFIG.lock().unwrap();

    if !list.is_null() && config.dlc_unlock_all {
        // Создаем список из 100 "купленных" DLC для теста
        let count = 100;
        let mut products = Vec::with_capacity(count);
        for i in 0..count {
            products.push(UPC_Product {
                appid: i as u32 + 1, // Просто последовательные ID
                product_type: 2, // DLC
                ownership_type: 1, // Owned
                state: 1, // Installed
            });
        }

        let product_list = Box::new(UPC_ProductList {
            count: count as u32,
            products: Box::into_raw(products.into_boxed_slice()) as *mut UPC_Product,
        });

        *list = Box::into_raw(product_list) as *mut c_void;
        log(&format!("Returned {} spoofed products", count));
    }
    0 // UPC_OK
}

#[no_mangle]
pub unsafe extern "C" fn UPC_ProductListFree(_context: *mut c_void, list: *mut c_void) -> i32 {
    log("UPC_ProductListFree");
    if !list.is_null() {
        let product_list = Box::from_raw(list as *mut UPC_ProductList);
        if !product_list.products.is_null() {
            let _ = Box::from_raw(std::slice::from_raw_parts_mut(product_list.products, product_list.count as usize));
        }
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn UPC_AchievementUnlock(_context: *mut c_void, _achievement_id: u32, _callback: *const c_void, _user_data: *const c_void) -> i32 { 0 }

#[no_mangle]
pub unsafe extern "C" fn UPC_AchievementListGet(_context: *mut c_void, _filter: *const i8, _flags: u32, _list: *mut c_void, _callback: *const c_void, _user_data: *const c_void) -> i32 { 0 }

#[no_mangle]
pub unsafe extern "C" fn UPC_OverlayShow(_context: *mut c_void, _section: u32, _callback: *const c_void, _user_data: *const c_void) -> i32 { 0 }

#[no_mangle]
pub unsafe extern "C" fn UPC_OverlayNotificationShow(_context: *mut c_void, _notification_id: u32) -> i32 { 0 }

#[no_mangle]
pub unsafe extern "C" fn UPC_RichPresenceSet(_context: *mut c_void, _presence_id: u32, _tokens: *const c_void) -> i32 { 0 }

#[no_mangle]
pub unsafe extern "C" fn UPC_StorageFileOpen(_context: *mut c_void, filename: *mut i8, _flags: u32, handle: *mut i32) -> i32 {
    let name = std::ffi::CStr::from_ptr(filename).to_string_lossy();
    log(&format!("UPC_StorageFileOpen(file={})", name));
    
    // Используем хеш имени файла как дескриптор или просто порядковый номер
    // В простейшем случае, мы можем просто вернуть 1, если файл "открыт"
    if !handle.is_null() {
        *handle = 1; // Всегда используем handle 1 для простоты
    }
    0 // UPC_OK
}

#[no_mangle]
pub unsafe extern "C" fn UPC_StorageFileClose(_context: *mut c_void, _handle: i32) -> i32 {
    log(&format!("UPC_StorageFileClose(handle={})", _handle));
    0
}

#[no_mangle]
pub unsafe extern "C" fn UPC_StorageFileRead(_context: *mut c_void, handle: i32, size: u32, offset: u32, buffer: *mut c_void, bytes_read: *mut i32, _callback: *const c_void, _user_data: *const c_void) -> i32 {
    log(&format!("UPC_StorageFileRead(handle={}, size={}, offset={})", handle, size, offset));
    
    let path = get_save_dir().join("upc_storage.bin");
    let mut read = 0;

    if path.exists() {
        if let Ok(data) = fs::read(&path) {
            let data_len = data.len();
            if (offset as usize) < data_len {
                let to_read = (size as usize).min(data_len - offset as usize);
                std::ptr::copy_nonoverlapping(data[offset as usize..].as_ptr() as *const c_void, buffer, to_read);
                read = to_read as i32;
            }
        }
    }

    if !bytes_read.is_null() {
        *bytes_read = read;
    }

    0 // UPC_OK
}

#[no_mangle]
pub unsafe extern "C" fn UPC_StorageFileWrite(_context: *mut c_void, handle: i32, buffer: *const c_void, size: i32, _callback: *const c_void, _user_data: *const c_void) -> i32 {
    log(&format!("UPC_StorageFileWrite(handle={}, size={})", handle, size));
    
    let path = get_save_dir().join("upc_storage.bin");
    let data = std::slice::from_raw_parts(buffer as *const u8, size as usize);
    
    let _ = fs::write(path, data);
    0 // UPC_OK
}

#[no_mangle]
pub unsafe extern "C" fn UPC_CPUScoreGet(_context: *mut c_void, _score: *mut u32) -> i32 {
    if !_score.is_null() {
        *_score = 1000;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn UPC_GPUScoreGet(_context: *mut c_void, _score: *mut u32, _confidence: *mut f32) -> i32 {
    if !_score.is_null() {
        *_score = 1000;
    }
    if !_confidence.is_null() {
        *_confidence = 1.0;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn UPC_LaunchApp(_context: *mut c_void, _app_id: u32, _callback: *const c_void) -> i32 { 0 }

#[no_mangle]
pub unsafe extern "C" fn UPC_FriendListGet(_context: *mut c_void, _flags: u32, _list: *mut c_void, _callback: *const c_void, _user_data: *const c_void) -> i32 { 0 }

#[no_mangle]
pub unsafe extern "C" fn UPC_UserPlayedWith(_context: *mut c_void, _account_id: *const i8) -> i32 { 0 }

#[no_mangle]
pub unsafe extern "C" fn UPC_StreamingCountryGet(_context: *mut c_void) -> *const i8 {
    b"US\0".as_ptr() as *const i8
}

#[no_mangle]
pub unsafe extern "C" fn UPC_StreamingInputEnable(_context: *mut c_void, _enable: i32) -> i32 { 0 }

#[no_mangle]
pub unsafe extern "C" fn UPC_TicketGet(_context: *mut c_void, buffer: *mut i8, size: u32) -> i32 {
    log("UPC_TicketGet");
    if !buffer.is_null() && size > 0 {
        let ticket = b"UPC_TICKET_DUMMY_DATA_VOID\0";
        let copy_len = ticket.len().min(size as usize);
        std::ptr::copy_nonoverlapping(ticket.as_ptr() as *const i8, buffer, copy_len);
        if copy_len < size as usize {
            *buffer.add(copy_len) = 0;
        }
    }
    0 // UPC_OK
}

// ============ DBDATA (Denuvo stub) - Minimal implementation ============
// Простая заглушка для обхода проверок Denuvo

#[no_mangle]
pub unsafe extern "C" fn dbdata_GetVersion() -> u32 {
    log("dbdata_GetVersion (Denuvo stub)");
    0x01000000 // Version 1.0.0.0
}

#[no_mangle]
pub unsafe extern "C" fn dbdata_Init(_param: *const c_void) -> i32 {
    log("dbdata_Init (Denuvo stub) - bypassed");
    0 // Success
}

#[no_mangle]
pub unsafe extern "C" fn dbdata_Uninit() -> i32 {
    log("dbdata_Uninit (Denuvo stub)");
    0
}

#[no_mangle]
pub unsafe extern "C" fn dbdata_CheckIntegrity() -> i32 {
    log("dbdata_CheckIntegrity (Denuvo stub) - always pass");
    1 // Always pass
}

#[no_mangle]
pub unsafe extern "C" fn dbdata_Validate(_data: *const c_void, _size: u32) -> i32 {
    log("dbdata_Validate (Denuvo stub) - always valid");
    1 // Always valid
}

#[no_mangle]
pub unsafe extern "C" fn dbdata_GetStatus() -> i32 {
    log("dbdata_GetStatus (Denuvo stub) - OK");
    0 // OK status
}

#[no_mangle]
pub unsafe extern "C" fn dbdata_IsActivated() -> i32 {
    log("dbdata_IsActivated (Denuvo stub) - always activated");
    1 // Always activated
}

#[no_mangle]
pub unsafe extern "C" fn dbdata_GetHardwareId(_buffer: *mut i8, _size: u32) -> i32 {
    log("dbdata_GetHardwareId (Denuvo stub)");
    if !_buffer.is_null() && _size > 0 {
        let hwid = b"VOID-HWID-00000000\0";
        std::ptr::copy_nonoverlapping(hwid.as_ptr() as *const i8, _buffer, hwid.len().min(_size as usize));
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn dbdata_GetMachineId(_buffer: *mut i8, _size: u32) -> i32 {
    log("dbdata_GetMachineId (Denuvo stub)");
    if !_buffer.is_null() && _size > 0 {
        let mid = b"VOID-MACHINE-ID-0000\0";
        std::ptr::copy_nonoverlapping(mid.as_ptr() as *const i8, _buffer, mid.len().min(_size as usize));
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn dbdata_Activate(_key: *const i8) -> i32 {
    log("dbdata_Activate (Denuvo stub) - auto-activated");
    0 // Success
}

#[no_mangle]
pub unsafe extern "C" fn dbdata_Deactivate() -> i32 {
    log("dbdata_Deactivate (Denuvo stub)");
    0
}

#[no_mangle]
pub unsafe extern "C" fn dbdata_GetActivationCount() -> i32 {
    log("dbdata_GetActivationCount (Denuvo stub)");
    1 // One activation
}

#[no_mangle]
pub unsafe extern "C" fn dbdata_GetActivationLimit() -> i32 {
    log("dbdata_GetActivationLimit (Denuvo stub)");
    999 // Unlimited
}

#[no_mangle]
pub unsafe extern "C" fn dbdata_GetDaysRemaining() -> i32 {
    log("dbdata_GetDaysRemaining (Denuvo stub)");
    9999 // Unlimited days
}

#[no_mangle]
pub unsafe extern "C" fn dbdata_IsTrialVersion() -> i32 {
    log("dbdata_IsTrialVersion (Denuvo stub)");
    0 // Not trial
}

#[no_mangle]
pub unsafe extern "C" fn dbdata_GetTrialDaysRemaining() -> i32 {
    log("dbdata_GetTrialDaysRemaining (Denuvo stub)");
    0 // Not applicable
}
