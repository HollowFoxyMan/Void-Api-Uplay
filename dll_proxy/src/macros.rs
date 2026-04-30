macro_rules! forward_void {
    ($name:expr, $sig:ty) => {
        {
            log(&format!("Calling: {}", $name));
            let h = get_handle();
            if h == 0 as HMODULE { 
                log(&format!("Error: Failed to get API handle for {}", $name));
                return; 
            }
            let proc = GetProcAddress(h, concat!($name, "\0").as_ptr() as *const i8);
            if proc.is_null() { 
                log(&format!("Error: Failed to find export {}", $name));
                return; 
            }
            let f: $sig = std::mem::transmute(proc);
            f
        }
    };
}
