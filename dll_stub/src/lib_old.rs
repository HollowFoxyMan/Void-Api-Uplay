use std::ffi::c_void;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;
use std::collections::HashMap;

static LOG_MUTEX: Mutex<()> = Mutex::new(());

fn log(msg: &str) {
    let _guard = LOG_MUTEX.lock().unwrap();
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open("void_uplay_api.log") {
        let _ = writeln!(f, "[MAIN] {}", msg);
    }
}

// Система полок - хранит обработчики для разных API
struct ApiShelf {
    name: &'static str,
    handlers: HashMap<String, fn(*const c_void, *const c_void) -> usize>,
}

impl ApiShelf {
    fn new(name: &'static str) -> Self {
        Self {
            name,
            handlers: HashMap::new(),
        }
    }

    fn register(&mut self, func_name: &str, handler: fn(*const c_void, *const c_void) -> usize) {
        self.handlers.insert(func_name.to_string(), handler);
    }

    fn call(&self, func_name: &str, arg1: *const c_void, arg2: *const c_void) -> Option<usize> {
        self.handlers.get(func_name).map(|handler| handler(arg1, arg2))
    }
}

// Глобальное хранилище полок
lazy_static::lazy_static! {
    static ref SHELVES: Mutex<HashMap<String, ApiShelf>> = {
        let mut shelves = HashMap::new();

        // Создаем полки для разных API
        let uplay_shelf = ApiShelf::new("UPLAY");
        let _upc_shelf = ApiShelf::new("UPC");

        shelves.insert("uplay_r1.dll".to_string(), uplay_shelf);
        shelves.insert("uplay_r1_loader.dll".to_string(), ApiShelf::new("UPLAY"));
        shelves.insert("uplay_r1_loader64.dll".to_string(), ApiShelf::new("UPLAY"));
        shelves.insert("uplay_r2_loader.dll".to_string(), ApiShelf::new("UPLAY"));
        shelves.insert("uplay_r2_loader64.dll".to_string(), ApiShelf::new("UPLAY"));
        shelves.insert("upc_r1_loader.dll".to_string(), ApiShelf::new("UPC"));
        shelves.insert("upc_r1_loader64.dll".to_string(), ApiShelf::new("UPC"));
        shelves.insert("upc_r2_loader.dll".to_string(), ApiShelf::new("UPC"));
        shelves.insert("upc_r2_loader64.dll".to_string(), ApiShelf::new("UPC"));

        Mutex::new(shelves)
    };
}

// Публичная функция для вызова из proxy DLL
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

    log(&format!("RouteCall: {} -> {}", source, func));

    let shelves = SHELVES.lock().unwrap();

    if let Some(shelf) = shelves.get(source.as_ref()) {
        log(&format!("Found shelf: {}", shelf.name));

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
            "UPLAY_GetInstallationError" => 0,
            "UPLAY_GetVersion" => 100,
            "UPLAY_GetLocale" => uplay_get_locale(arg1 as *mut i8, arg2 as u32),
            "UPLAY_GetTime" => uplay_get_time(arg1 as *mut u64),
            "UPLAY_SetGameSession" => 0,
            "UPLAY_ClearGameSession" => 0,
            "UPLAY_HasOverlappedOperationCompleted" => uplay_has_overlapped_completed(arg1 as *const c_void),
            "UPLAY_GetOverlappedOperationResult" => uplay_get_overlapped_result(arg1 as *const c_void, arg2 as *mut c_void),

            // UPLAY User
            "UPLAY_USER_GetNameUtf8" => uplay_user_get_name_utf8(arg1 as *mut i8, arg2 as u32),
            "UPLAY_USER_GetAccountIdUtf8" => uplay_user_get_account_id_utf8(arg1 as *mut i8, arg2 as u32),
            "UPLAY_USER_GetUsernameUtf8" => uplay_user_get_name_utf8(arg1 as *mut i8, arg2 as u32),
            "UPLAY_USER_GetEmailUtf8" => uplay_user_get_email_utf8(arg1 as *mut i8, arg2 as u32),
            "UPLAY_USER_GetPasswordUtf8" => uplay_user_get_password_utf8(arg1 as *mut i8, arg2 as u32),
            "UPLAY_USER_GetTicketUtf8" => uplay_user_get_ticket_utf8(arg1 as *mut i8, arg2 as u32),
            "UPLAY_USER_IsConnected" => 1,
            "UPLAY_USER_IsOwned" => 1,
            "UPLAY_USER_IsInOfflineMode" => 1,
            "UPLAY_USER_GetCredentials" => uplay_user_get_credentials(arg1 as *mut c_void, arg2 as *mut c_void),
            "UPLAY_USER_SetGameSession" => 0,
            "UPLAY_USER_ClearGameSession" => 1,
            "UPLAY_USER_GetCdKeys" => 1,
            "UPLAY_USER_ReleaseCdKeyList" => 1,
            "UPLAY_USER_GetCdKeyUtf8" => 0,
            "UPLAY_USER_ConsumeItem" => 0,
            "UPLAY_USER_GetConsumeItem" => 0,
            "UPLAY_USER_GetConsumableItems" => 0,
            "UPLAY_USER_ReleaseConsumeItemResult" => 0,
            "UPLAY_USER_GetGPUScoreConfidenceLevel" => 0,
            "UPLAY_USER_GetGPUScore" => 0,
            "UPLAY_USER_GetCPUScore" => 0,

            // UPLAY Achievements
            "UPLAY_ACH_EarnAchievement" | "UPLAY_ACH_EarnAchivement" => uplay_ach_earn(arg1 as u32, arg2 as *mut c_void),
            "UPLAY_ACH_GetAchievements" => 0,
            "UPLAY_ACH_GetAchievementImage" => 0,
            "UPLAY_ACH_Write" => 0,
            "UPLAY_ACH_ReleaseAchievementList" => 1,
            "UPLAY_ACHIEVEMENT_Read" => 1,
            "UPLAY_ACHIEVEMENT_Write" => 1,

            // UPLAY Save
            "UPLAY_SAVE_GetSavegames" => uplay_save_get_savegames(arg1 as *mut *mut c_void, arg2 as *mut c_void),
            "UPLAY_SAVE_Open" => uplay_save_open(arg1 as u32, arg2 as u32, arg1 as *mut u32, arg2 as *mut c_void),
            "UPLAY_SAVE_Read" => 0,
            "UPLAY_SAVE_Write" => 0,
            "UPLAY_SAVE_Close" => 1,
            "UPLAY_SAVE_Remove" => uplay_save_remove(arg1 as u32, arg2 as *mut c_void),
            "UPLAY_SAVE_SetName" => 1,
            "UPLAY_SAVE_ReleaseGameList" => 1,
            "UPLAY_SAVE_GetLocalPath" => 1,
            "UPLAY_SAVE_GetCloudPath" => 1,

            // UPLAY Products
            "UPLAY_PRODUCT_IsOwned" => 1,
            "UPLAY_PRODUCT_GetProductList" | "UPLAY_PRODUCT_Get_Product_list" => 1,
            "UPLAY_PRODUCT_ReleaseProductList" | "UPLAY_PRODUCT_ReleaseProduct_list" => 1,
            "UPLAY_PRODUCT_GetDLCList" => 1,

            // UPLAY Overlay
            "UPLAY_OVERLAY_SetShopUrl" => uplay_overlay_set_shop_url(arg1 as *const i8, arg2 as *mut c_void),
            "UPLAY_OVERLAY_ShowShopUrl" => 0,
            "UPLAY_OVERLAY_Show" => 1,
            "UPLAY_OVERLAY_SetVisibility" => 1,
            "UPLAY_OVERLAY_ShowNotification" => 0,
            "UPLAY_GetOverlayVisibility" => 0,

            // UPLAY Friends
            "UPLAY_FRIENDS_Init" => 0,
            "UPLAY_FRIENDS_GetFriendList" => 0,
            "UPLAY_FRIENDS_RequestFriendship" => 0,
            "UPLAY_FRIENDS_IsFriend" => 0,
            "UPLAY_FRIENDS_AddToBlackList" => 0,
            "UPLAY_FRIENDS_IsBlackListed" => 0,
            "UPLAY_FRIENDS_ShowFriendSelectionUI" => 0,
            "UPLAY_FRIENDS_EnableFriendMenuItem" => 0,
            "UPLAY_FRIENDS_DisableFriendMenuItem" => 0,
            "UPLAY_FRIENDS_InviteToGame" => 0,
            "UPLAY_FRIEND_GetFriendList" => 1,
            "UPLAY_FRIEND_Invite" => 1,
            "UPLAY_FRIEND_GetFriendPresence" => 1,
            "UPLAY_FRIEND_ShowFriendList" => 1,

            // UPLAY Avatar
            "UPLAY_AVATAR_GetBitmap" => 0,
            "UPLAY_AVATAR_GetAvatarIdForCurrentUser" => 0,
            "UPLAY_AVATAR_Get" => 0,

            // UPLAY Party
            "UPLAY_PARTY_Init" => 1,
            "UPLAY_PARTY_InviteToParty" => 0,
            "UPLAY_PARTY_RespondToGameInvite" => 0,
            "UPLAY_PARTY_ShowGameInviteOverlayUI" => 0,
            "UPLAY_PARTY_GetInGameMemberList" => 0,
            "UPLAY_PARTY_GetFullMemberList" => 0,
            "UPLAY_PARTY_SetUserData" => 0,
            "UPLAY_PARTY_IsInParty" => 0,
            "UPLAY_PARTY_IsPartyLeader" => 0,
            "UPLAY_PARTY_PromoteToLeader" => 0,
            "UPLAY_PARTY_InvitePartyToGame" => 0,
            "UPLAY_PARTY_EnablePartyMemberMenuItem" => 0,
            "UPLAY_PARTY_DisablePartyMemberMenuItem" => 0,
            "UPLAY_PARTY_SetGuest" => 0,

            // UPLAY Presence
            "UPLAY_PRESENCE_SetPresence" => 1,

            // UPLAY Metadata
            "UPLAY_METADATA_SetSingleEventTag" => 0,
            "UPLAY_METADATA_SetContinuousTag" => 0,
            "UPLAY_METADATA_ClearContinuousTag" => 0,

            // UPLAY Options
            "UPLAY_OPTIONS_Enumerate" => 0,
            "UPLAY_OPTIONS_Set" => 0,
            "UPLAY_OPTIONS_Apply" => 0,
            "UPLAY_OPTIONS_ReleaseKeyValueList" => 0,
            "UPLAY_OPTIONS_SetInGameState" => 1,

            // UPLAY Win/Rewards
            "UPLAY_WIN_GetRewards" => 0,
            "UPLAY_WIN_RefreshActions" => 1,
            "UPLAY_WIN_SetActionsCompleted" => 1,
            "UPLAY_WIN_ReleaseRewardList" => 1,
            "UPLAY_WIN_GetRewardList" => 0,

            // UPLAY Storage
            "UPLAY_STORAGE_Read" => 1,
            "UPLAY_STORAGE_Write" => 1,

            // UPLAY Monetization
            "UPLAY_MONETIZATION_GetCurrency" => 1,

            // UPC Core
            "UPC_Init" => upc_init(arg1 as u32, arg2 as i32),
            "UPC_Uninit" => { upc_uninit(); 0 },
            "UPC_ContextCreate" => upc_context_create(arg1 as u32, arg2) as usize,
            "UPC_ContextFree" => upc_context_free(arg1 as *mut c_void),
            "UPC_Update" => upc_update(arg1 as *mut c_void),
            "UPC_ErrorToString" => b"Unknown\0".as_ptr() as usize,
            "UPC_Cancel" => 0,

            // UPC Events
            "UPC_EventNextPeek" => -6isize as usize,
            "UPC_EventNextPoll" => -6isize as usize,
            "UPC_EventRegisterHandler" => 0,
            "UPC_EventUnregisterHandler" => 0,

            // UPC User Info
            "UPC_EmailGet" => upc_email_get(arg1 as *mut c_void) as usize,
            "UPC_EmailGet_Extended" => upc_email_get_extended(arg1 as *mut c_void, arg2 as *mut *const i8),
            "UPC_IdGet" => upc_id_get(arg1 as *mut c_void) as usize,
            "UPC_IdGet_Extended" => upc_id_get_extended(arg1 as *mut c_void, arg2 as *mut *const i8),
            "UPC_NameGet" => upc_name_get(arg1 as *mut c_void) as usize,
            "UPC_NameGet_Extended" => upc_name_get_extended(arg1 as *mut c_void, arg2 as *mut *const i8),
            "UPC_InstallLanguageGet" => upc_language_get(arg1 as *mut c_void) as usize,
            "UPC_InstallLanguageGet_Extended" => upc_language_get_extended(arg1 as *mut c_void, arg2 as *mut *const i8),
            "UPC_ApplicationIdGet" => 0,
            "UPC_TicketGet" => b"\0".as_ptr() as usize,
            "UPC_TicketGet_Extended" => 0,

            // UPC User
            "UPC_UserGet" => upc_user_get(arg1 as *mut c_void, arg2 as *mut i8, arg1 as *mut *mut c_void),
            "UPC_UserFree" => upc_user_free(arg1 as *mut c_void, arg2 as *mut c_void),
            "UPC_UserPlayedWithAdd" => 0,
            "UPC_UserPlayedWithAdd_Extended" => 0,

            // UPC Achievements
            "UPC_AchievementUnlock" => 0,
            "UPC_AchievementListGet" => 0,
            "UPC_AchievementListFree" => 0,
            "UPC_AchievementImageGet" => 0,
            "UPC_AchievementImageFree" => 0,

            // UPC Products
            "UPC_ProductListGet" => upc_product_list_get(arg1 as *mut c_void, arg2 as *const i8, arg1 as u32, arg2 as *mut *mut c_void),
            "UPC_ProductListFree" => upc_product_list_free(arg1 as *mut c_void, arg2 as *mut c_void),
            "UPC_ProductConsume" => 0,
            "UPC_ProductConsumeSignatureFree" => 0,
            "UPC_LaunchApp" | "CPC_LaunchApp" => 1,
            "UPC_IsCrossBootAllowed" => upc_is_crossboot_allowed(arg1 as *mut c_void, arg2 as u32, arg1 as *mut i32),

            // UPC Overlay
            "UPC_OverlayShow" => 0,
            "UPC_OverlayNotificationShow" => 0,
            "UPC_OverlayNotificationShow_Extended" => 0,
            "UPC_OverlayFriendSelectionShow" => 0,
            "UPC_OverlayFriendSelectionFree" => 0,
            "UPC_OverlayFriendInvitationShow" => 0,
            "UPC_OverlayFriendInvitationShow_Extended" => 0,
            "UPC_OverlayMicroAppShow" => 0,

            // UPC Rich Presence
            "UPC_RichPresenceSet" => 0,
            "UPC_RichPresenceSet_Extended" => 0,

            // UPC Multiplayer
            "UPC_MultiplayerSessionGet" => 0,
            "UPC_MultiplayerSessionSet" => 0,
            "UPC_MultiplayerSessionSet_Extended" => 0,
            "UPC_MultiplayerSessionClear" => 0,
            "UPC_MultiplayerSessionClear_Extended" => 0,
            "UPC_MultiplayerSessionFree" => 0,
            "UPC_MultiplayerInvite" => 0,
            "UPC_MultiplayerInviteAnswer" => 0,

            // UPC Friends
            "UPC_FriendListGet" => -0xDisize as usize,
            "UPC_FriendListFree" => 0,
            "UPC_FriendAdd" => 0,
            "UPC_FriendRemove" => 0,
            "UPC_FriendCheck" => 0,
            "UPC_FriendCheck_Extended" => 0,
            "UPC_BlacklistAdd" => 0,
            "UPC_BlacklistHas" => 0,
            "UPC_BlacklistHas_Extended" => 0,

            // UPC Avatar
            "UPC_AvatarGet" => 0,
            "UPC_AvatarFree" => 0,

            // UPC System Info
            "UPC_CPUScoreGet" => upc_cpu_score_get(arg1 as *mut c_void, arg2 as *mut u32),
            "UPC_GPUScoreGet" => upc_gpu_score_get(arg1 as *mut c_void, arg2 as *mut u32, arg1 as *mut f32),

            // UPC Storage
            "UPC_StorageFileOpen" => 0,
            "UPC_StorageFileClose" => 0,
            "UPC_StorageFileRead" => 0,
            "UPC_StorageFileWrite" => 0,
            "UPC_StorageFileDelete" => 0,
            "UPC_StorageFileListGet" => upc_storage_file_list_get(arg1 as *mut c_void, arg2 as *mut *mut c_void),
            "UPC_StorageFileListFree" => upc_storage_file_list_free(arg1 as *mut c_void, arg2 as *mut c_void),

            // UPC Install Chunks
            "UPC_InstallChunkListGet" => 0,
            "UPC_InstallChunkListFree" => 0,
            "UPC_InstallChunksOrderUpdate" => 0,
            "UPC_InstallChunksOrderUpdate_Extended" => 0,
            "UPC_InstallChunksPresenceCheck" => 0,

            // UPC Store
            "UPC_StoreIsEnabled" => 1,
            "UPC_StoreIsEnabled_Extended" => upc_store_is_enabled_extended(arg1 as *mut c_void, arg2 as *mut i32),
            "UPC_StoreLanguageSet" => 0,
            "UPC_StoreCheckout" => 0,
            "UPC_StoreProductListGet" => 0,
            "UPC_StoreProductListFree" => 0,
            "UPC_StoreProductDetailsShow" => 0,
            "UPC_StoreProductsShow" => 0,
            "UPC_StorePartnerGet" => 0,
            "UPC_StorePartnerGet_Extended" => 0,

            // UPC Streaming
            "UPC_StreamingTypeGet" => 0x200,
            "UPC_StreamingCurrentUserCountryGet" => 0,
            "UPC_StreamingCurrentUserCountryFree" => 0,
            "UPC_StreamingDeviceTypeGet" => 0,
            "UPC_StreamingInputTypeGet" => 0,
            "UPC_StreamingInputGamepadTypeGet" => 0,
            "UPC_StreamingNetworkDelayForInputGet" => 0,
            "UPC_StreamingNetworkDelayForVideoGet" => 0,
            "UPC_StreamingNetworkDelayRoundtripGet" => 0,
            "UPC_StreamingResolutionGet" => 0,
            "UPC_StreamingResolutionFree" => 0,

            // UPC Browser
            "UPC_ShowBrowserUrl" => 0,

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

// ============ UPLAY API Implementation ============

fn uplay_init() -> usize {
    log("UPLAY_Init");
    1
}

fn uplay_start(id: u32, flags: u32) -> usize {
    log(&format!("UPLAY_Start({}, {})", id, flags));
    1
}

fn uplay_startup(id: u32, ver: u32, _lang: *const i8) -> usize {
    log(&format!("UPLAY_Startup({}, {})", id, ver));
    1
}

fn uplay_user_get_name_utf8(buffer: *mut i8, size: u32) -> usize {
    log("UPLAY_USER_GetNameUtf8");
    unsafe {
        let name = b"VoidUser\0";
        std::ptr::copy_nonoverlapping(name.as_ptr() as *const i8, buffer, name.len().min(size as usize));
    }
    1
}

fn uplay_user_get_account_id_utf8(buffer: *mut i8, size: u32) -> usize {
    log("UPLAY_USER_GetAccountIdUtf8");
    unsafe {
        let id = b"5d3e2202-106b-46fc-b71f-000e5e593556\0";
        std::ptr::copy_nonoverlapping(id.as_ptr() as *const i8, buffer, id.len().min(size as usize));
    }
    1
}

fn uplay_user_get_email_utf8(buffer: *mut i8, size: u32) -> usize {
    log("UPLAY_USER_GetEmailUtf8");
    unsafe {
        let email = b"offline@void.local\0";
        std::ptr::copy_nonoverlapping(email.as_ptr() as *const i8, buffer, email.len().min(size as usize));
    }
    1
}

fn uplay_user_get_password_utf8(buffer: *mut i8, size: u32) -> usize {
    log("UPLAY_USER_GetPasswordUtf8");
    unsafe {
        let pwd = b"\0";
        std::ptr::copy_nonoverlapping(pwd.as_ptr() as *const i8, buffer, pwd.len().min(size as usize));
    }
    1
}

fn uplay_user_get_ticket_utf8(buffer: *mut i8, size: u32) -> usize {
    log("UPLAY_USER_GetTicketUtf8");
    unsafe {
        let ticket = b"SESSION-OFFLINE-VOID-0001\0";
        std::ptr::copy_nonoverlapping(ticket.as_ptr() as *const i8, buffer, ticket.len().min(size as usize));
    }
    1
}

fn uplay_user_get_credentials(_user_credentials: *mut c_void, overlapped: *mut c_void) -> usize {
    log("UPLAY_USER_GetCredentials");
    if !overlapped.is_null() {
        unsafe {
            let ovl = overlapped as *mut UplayOverlapped;
            (*ovl).set_completed();
        }
    }
    0
}

fn uplay_get_locale(buffer: *mut i8, size: u32) -> usize {
    log("UPLAY_GetLocale");
    unsafe {
        let lang = b"en-US\0";
        std::ptr::copy_nonoverlapping(lang.as_ptr() as *const i8, buffer, lang.len().min(size as usize));
    }
    1
}

fn uplay_get_time(time_ptr: *mut u64) -> usize {
    log("UPLAY_GetTime");
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
                *(result as *mut i32) = (*ovl).reserved;
                return 1;
            }
        }
    }
    0
}

fn uplay_ach_earn(_achievement_id: u32, overlapped: *mut c_void) -> usize {
    log("UPLAY_ACH_EarnAchievement");
    if !overlapped.is_null() {
        unsafe {
            let ovl = overlapped as *mut UplayOverlapped;
            (*ovl).set_completed();
        }
    }
    1
}

fn uplay_save_get_savegames(_games_list: *mut *mut c_void, overlapped: *mut c_void) -> usize {
    log("UPLAY_SAVE_GetSavegames");
    if !overlapped.is_null() {
        unsafe {
            let ovl = overlapped as *mut UplayOverlapped;
            (*ovl).set_completed();
        }
    }
    1
}

fn uplay_save_open(_slot_id: u32, _mode: u32, save_handle: *mut u32, overlapped: *mut c_void) -> usize {
    log("UPLAY_SAVE_Open");
    if !save_handle.is_null() {
        unsafe {
            *save_handle = _slot_id;
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

fn uplay_save_remove(_slot_id: u32, overlapped: *mut c_void) -> usize {
    log("UPLAY_SAVE_Remove");
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

#[repr(C)]
pub struct UplayOverlapped {
    pub unk: u32,
    pub is_completed: u32,
    pub reserved: i32,
}

impl UplayOverlapped {
    pub fn set_completed(&mut self) {
        self.unk += 1;
        self.is_completed = 1;
        self.reserved = 0;
    }
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
    log(&format!("UPC_Init({}, {})", version, appid));
    0
}

fn upc_uninit() {
    log("UPC_Uninit");
}

fn upc_context_create(version: u32, _settings: *const c_void) -> *mut UPC_Context {
    log(&format!("UPC_ContextCreate({})", version));
    let ctx = Box::new(UPC_Context {
        version: 0x20220811,
        email: b"offline@localhost\0".as_ptr() as *const i8,
        name: b"VoidUser\0".as_ptr() as *const i8,
        id: b"00000000-0000-0000-0000-000000000000\0".as_ptr() as *const i8,
        language: b"en-US\0".as_ptr() as *const i8,
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
    log("UPC_EmailGet");
    if context.is_null() {
        return b"\0".as_ptr() as *const i8;
    }
    unsafe { (*(context as *mut UPC_Context)).email }
}

fn upc_email_get_extended(context: *mut c_void, out_email: *mut *const i8) -> usize {
    log("UPC_EmailGet_Extended");
    if !context.is_null() && !out_email.is_null() {
        unsafe {
            *out_email = (*(context as *mut UPC_Context)).email;
        }
    }
    0
}

fn upc_id_get(context: *mut c_void) -> *const i8 {
    log("UPC_IdGet");
    if context.is_null() {
        return b"\0".as_ptr() as *const i8;
    }
    unsafe { (*(context as *mut UPC_Context)).id }
}

fn upc_id_get_extended(context: *mut c_void, out_id: *mut *const i8) -> usize {
    log("UPC_IdGet_Extended");
    if !context.is_null() && !out_id.is_null() {
        unsafe {
            *out_id = (*(context as *mut UPC_Context)).id;
        }
    }
    0
}

fn upc_name_get(context: *mut c_void) -> *const i8 {
    log("UPC_NameGet");
    if context.is_null() {
        return b"\0".as_ptr() as *const i8;
    }
    unsafe { (*(context as *mut UPC_Context)).name }
}

fn upc_name_get_extended(context: *mut c_void, out_name: *mut *const i8) -> usize {
    log("UPC_NameGet_Extended");
    if !context.is_null() && !out_name.is_null() {
        unsafe {
            *out_name = (*(context as *mut UPC_Context)).name;
        }
    }
    0
}

fn upc_language_get(context: *mut c_void) -> *const i8 {
    log("UPC_InstallLanguageGet");
    if context.is_null() {
        return b"en-US\0".as_ptr() as *const i8;
    }
    unsafe { (*(context as *mut UPC_Context)).language }
}

fn upc_language_get_extended(context: *mut c_void, out_lang: *mut *const i8) -> usize {
    log("UPC_InstallLanguageGet_Extended");
    if !context.is_null() && !out_lang.is_null() {
        unsafe {
            *out_lang = (*(context as *mut UPC_Context)).language;
        }
    }
    0
}

#[repr(C)]
pub struct UPC_User {
    pub id_utf8: *mut i8,
    pub name_utf8: *mut i8,
    pub relationship: u32,
}

fn upc_user_get(_context: *mut c_void, _user_id_utf8: *mut i8, out_user: *mut *mut c_void) -> usize {
    log("UPC_UserGet");
    if !out_user.is_null() {
        let user = Box::new(UPC_User {
            id_utf8: std::ptr::null_mut(),
            name_utf8: std::ptr::null_mut(),
            relationship: 0,
        });
        unsafe {
            *out_user = Box::into_raw(user) as *mut c_void;
        }
    }
    0x10000
}

fn upc_user_free(_context: *mut c_void, user: *mut c_void) -> usize {
    log("UPC_UserFree");
    if !user.is_null() {
        unsafe {
            let _ = Box::from_raw(user as *mut UPC_User);
        }
    }
    0
}

#[repr(C)]
pub struct UPC_ProductList {
    pub count: u32,
    pub list: *mut *mut c_void,
}

fn upc_product_list_get(_context: *mut c_void, _user_id_utf8: *const i8, _filter: u32, out_list: *mut *mut c_void) -> usize {
    log("UPC_ProductListGet");
    if !out_list.is_null() {
        let list = Box::new(UPC_ProductList {
            count: 0,
            list: std::ptr::null_mut(),
        });
        unsafe {
            *out_list = Box::into_raw(list) as *mut c_void;
        }
    }
    0x10000
}

fn upc_product_list_free(_context: *mut c_void, list: *mut c_void) -> usize {
    log("UPC_ProductListFree");
    if !list.is_null() {
        unsafe {
            let _ = Box::from_raw(list as *mut UPC_ProductList);
        }
    }
    0
}

fn upc_is_crossboot_allowed(_context: *mut c_void, _product_id: u32, out_allowed: *mut i32) -> usize {
    log("UPC_IsCrossBootAllowed");
    if !out_allowed.is_null() {
        unsafe {
            *out_allowed = 0;
        }
    }
    0
}

fn upc_cpu_score_get(_context: *mut c_void, out_score: *mut u32) -> usize {
    log("UPC_CPUScoreGet");
    if !out_score.is_null() {
        unsafe {
            *out_score = 0x1000;
        }
    }
    0
}

fn upc_gpu_score_get(_context: *mut c_void, out_score: *mut u32, out_confidence: *mut f32) -> usize {
    log("UPC_GPUScoreGet");
    if !out_score.is_null() {
        unsafe {
            *out_score = 0x1000;
        }
    }
    if !out_confidence.is_null() {
        unsafe {
            *out_confidence = 1.0;
        }
    }
    0
}

#[repr(C)]
pub struct UPC_StorageFileList {
    pub count: u32,
    pub list: *mut *mut c_void,
}

fn upc_storage_file_list_get(_context: *mut c_void, out_list: *mut *mut c_void) -> usize {
    log("UPC_StorageFileListGet");
    if !out_list.is_null() {
        let list = Box::new(UPC_StorageFileList {
            count: 0,
            list: std::ptr::null_mut(),
        });
        unsafe {
            *out_list = Box::into_raw(list) as *mut c_void;
        }
    }
    0
}

fn upc_storage_file_list_free(_context: *mut c_void, list: *mut c_void) -> usize {
    log("UPC_StorageFileListFree");
    if !list.is_null() {
        unsafe {
            let _ = Box::from_raw(list as *mut UPC_StorageFileList);
        }
    }
    0
}

fn upc_store_is_enabled_extended(_context: *mut c_void, out_enabled: *mut i32) -> usize {
    log("UPC_StoreIsEnabled_Extended");
    if !out_enabled.is_null() {
        unsafe {
            *out_enabled = 1;
        }
    }
    0
}

// ============ Direct Export Functions (для обратной совместимости) ============

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
pub unsafe extern "C" fn UPLAY_USER_GetNameUtf8(buffer: *mut i8, size: u32) -> usize {
    uplay_user_get_name_utf8(buffer, size)
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_USER_GetAccountIdUtf8(buffer: *mut i8, size: u32) -> usize {
    uplay_user_get_account_id_utf8(buffer, size)
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
pub unsafe extern "C" fn UPLAY_USER_GetEmailUtf8(buffer: *mut i8, size: u32) -> usize {
    uplay_user_get_email_utf8(buffer, size)
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_USER_GetPasswordUtf8(buffer: *mut i8, size: u32) -> usize {
    uplay_user_get_password_utf8(buffer, size)
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_USER_GetTicketUtf8(buffer: *mut i8, size: u32) -> usize {
    uplay_user_get_ticket_utf8(buffer, size)
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_GetLocale(buffer: *mut i8, size: u32) -> usize {
    uplay_get_locale(buffer, size)
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_GetTime(time_ptr: *mut u64) -> usize {
    uplay_get_time(time_ptr)
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_GetVersion() -> usize {
    100
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_HasOverlappedOperationCompleted(overlapped: *const c_void) -> usize {
    uplay_has_overlapped_completed(overlapped)
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_GetOverlappedOperationResult(overlapped: *const c_void, result: *mut c_void) -> usize {
    uplay_get_overlapped_result(overlapped, result)
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_Release(_list: *mut c_void) -> usize {
    1
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_GetNextEvent(_out_event: *mut isize) -> usize {
    0
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_GetLastError(_error_string: *const i8) -> usize {
    0
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_ACH_EarnAchievement(achievement_id: u32, overlapped: *mut c_void) -> usize {
    uplay_ach_earn(achievement_id, overlapped)
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_ACH_EarnAchivement(achievement_id: u32, overlapped: *mut c_void) -> usize {
    uplay_ach_earn(achievement_id, overlapped)
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_SAVE_GetSavegames(games_list: *mut *mut c_void, overlapped: *mut c_void) -> usize {
    uplay_save_get_savegames(games_list, overlapped)
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_SAVE_Open(slot_id: u32, mode: u32, save_handle: *mut u32, overlapped: *mut c_void) -> usize {
    uplay_save_open(slot_id, mode, save_handle, overlapped)
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_SAVE_Close(_handle: u32) -> usize {
    1
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_SAVE_Remove(slot_id: u32, overlapped: *mut c_void) -> usize {
    uplay_save_remove(slot_id, overlapped)
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_OVERLAY_SetShopUrl(url_utf8: *const i8, overlapped: *mut c_void) -> usize {
    uplay_overlay_set_shop_url(url_utf8, overlapped)
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_OVERLAY_Show(_section: u32) -> usize {
    1
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_GetOverlayVisibility() -> usize {
    0
}

#[no_mangle]
pub unsafe extern "C" fn UPLAY_PRODUCT_IsOwned(_product_id: u32) -> usize {
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
pub unsafe extern "C" fn UPC_ErrorToString(_error: i32) -> *const i8 {
    b"Unknown\0".as_ptr() as *const i8
}

#[no_mangle]
pub unsafe extern "C" fn UPC_EmailGet(context: *mut UPC_Context) -> *const i8 {
    upc_email_get(context as *mut c_void)
}

#[no_mangle]
pub unsafe extern "C" fn UPC_EmailGet_Extended(context: *mut UPC_Context, out_email: *mut *const i8) -> i32 {
    upc_email_get_extended(context as *mut c_void, out_email) as i32
}

#[no_mangle]
pub unsafe extern "C" fn UPC_IdGet(context: *mut UPC_Context) -> *const i8 {
    upc_id_get(context as *mut c_void)
}

#[no_mangle]
pub unsafe extern "C" fn UPC_IdGet_Extended(context: *mut UPC_Context, out_id: *mut *const i8) -> i32 {
    upc_id_get_extended(context as *mut c_void, out_id) as i32
}

#[no_mangle]
pub unsafe extern "C" fn UPC_NameGet(context: *mut UPC_Context) -> *const i8 {
    upc_name_get(context as *mut c_void)
}

#[no_mangle]
pub unsafe extern "C" fn UPC_NameGet_Extended(context: *mut UPC_Context, out_name: *mut *const i8) -> i32 {
    upc_name_get_extended(context as *mut c_void, out_name) as i32
}

#[no_mangle]
pub unsafe extern "C" fn UPC_InstallLanguageGet(context: *mut UPC_Context) -> *const i8 {
    upc_language_get(context as *mut c_void)
}

#[no_mangle]
pub unsafe extern "C" fn UPC_InstallLanguageGet_Extended(context: *mut UPC_Context, out_lang: *mut *const i8) -> i32 {
    upc_language_get_extended(context as *mut c_void, out_lang) as i32
}

#[no_mangle]
pub unsafe extern "C" fn UPC_EventNextPeek(_context: *mut UPC_Context, _event: *mut c_void) -> i32 {
    -6
}

#[no_mangle]
pub unsafe extern "C" fn UPC_EventNextPoll(_context: *mut UPC_Context, _event: *mut c_void) -> i32 {
    -6
}

#[no_mangle]
pub unsafe extern "C" fn UPC_UserGet(context: *mut UPC_Context, user_id_utf8: *mut i8, out_user: *mut *mut UPC_User, _callback: *const c_void, _callback_data: *const c_void) -> i32 {
    upc_user_get(context as *mut c_void, user_id_utf8, out_user as *mut *mut c_void) as i32
}

#[no_mangle]
pub unsafe extern "C" fn UPC_UserFree(context: *mut UPC_Context, user: *mut UPC_User) -> i32 {
    upc_user_free(context as *mut c_void, user as *mut c_void) as i32
}

#[no_mangle]
pub unsafe extern "C" fn UPC_ProductListGet(context: *mut UPC_Context, user_id_utf8: *const i8, filter: u32, out_list: *mut *mut UPC_ProductList, _callback: *const c_void, _callback_data: *const c_void) -> i32 {
    upc_product_list_get(context as *mut c_void, user_id_utf8, filter, out_list as *mut *mut c_void) as i32
}

#[no_mangle]
pub unsafe extern "C" fn UPC_ProductListFree(context: *mut UPC_Context, list: *mut UPC_ProductList) -> i32 {
    upc_product_list_free(context as *mut c_void, list as *mut c_void) as i32
}

#[no_mangle]
pub unsafe extern "C" fn UPC_AchievementUnlock(_context: *mut UPC_Context, _achievement_id: u32, _callback: *const c_void, _callback_data: *const c_void) -> i32 {
    0
}

#[no_mangle]
pub unsafe extern "C" fn UPC_OverlayShow(_context: *mut UPC_Context, _section: u32, _callback: *const c_void, _callback_data: *const c_void) -> i32 {
    0
}

#[no_mangle]
pub unsafe extern "C" fn UPC_OverlayNotificationShow(_context: *mut UPC_Context, _notification_id: u32) -> i32 {
    0
}

#[no_mangle]
pub unsafe extern "C" fn UPC_RichPresenceSet(_context: *mut UPC_Context, _id: u32, _token_list: *const c_void) -> i32 {
    0
}

#[no_mangle]
pub unsafe extern "C" fn UPC_IsCrossBootAllowed(context: *mut UPC_Context, product_id: u32, out_allowed: *mut i32, _a: *const c_void, _b: *const c_void) -> i32 {
    upc_is_crossboot_allowed(context as *mut c_void, product_id, out_allowed) as i32
}

#[no_mangle]
pub unsafe extern "C" fn UPC_CPUScoreGet(context: *mut UPC_Context, out_score: *mut u32) -> i32 {
    upc_cpu_score_get(context as *mut c_void, out_score) as i32
}

#[no_mangle]
pub unsafe extern "C" fn UPC_GPUScoreGet(context: *mut UPC_Context, out_score: *mut u32, out_confidence: *mut f32) -> i32 {
    upc_gpu_score_get(context as *mut c_void, out_score, out_confidence) as i32
}

#[no_mangle]
pub unsafe extern "C" fn UPC_StorageFileListGet(context: *mut UPC_Context, out_list: *mut *mut UPC_StorageFileList) -> i32 {
    upc_storage_file_list_get(context as *mut c_void, out_list as *mut *mut c_void) as i32
}

#[no_mangle]
pub unsafe extern "C" fn UPC_StorageFileListFree(context: *mut UPC_Context, list: *mut UPC_StorageFileList) -> i32 {
    upc_storage_file_list_free(context as *mut c_void, list as *mut c_void) as i32
}

#[no_mangle]
pub unsafe extern "C" fn UPC_StoreIsEnabled(_context: *mut UPC_Context) -> i32 {
    1
}

#[no_mangle]
pub unsafe extern "C" fn UPC_StoreIsEnabled_Extended(context: *mut UPC_Context, out_enabled: *mut i32) -> i32 {
    upc_store_is_enabled_extended(context as *mut c_void, out_enabled) as i32
}

#[no_mangle]
pub unsafe extern "C" fn UPC_LaunchApp(_context: *mut UPC_Context, _product_id: u32, _must_be_zero: *const c_void) -> i32 {
    1
}

#[no_mangle]
pub unsafe extern "C" fn UPC_FriendListGet(_context: *mut UPC_Context, _online_filter: u32, _out_list: *mut c_void, _callback: *const c_void, _callback_data: *const c_void) -> i32 {
    -0xD
}

#[no_mangle]
pub unsafe extern "C" fn UPC_StreamingTypeGet(_context: *mut UPC_Context, _out_type: *mut u32, _callback: *const c_void, _callback_data: *const c_void) -> i32 {
    0x200
}

#[no_mangle]
pub unsafe extern "system" fn DllMain(_hinst: *const u8, reason: u32, _reserved: *const u8) -> i32 {
    match reason {
        1 => log("DllMain: PROCESS_ATTACH"),
        0 => log("DllMain: PROCESS_DETACH"),
        _ => {}
    }
    1
}
