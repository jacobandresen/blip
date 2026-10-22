//! Browser imports and user files.

#[cfg(target_arch = "wasm32")]
extern "C" {
    pub(crate) fn blip_basic_source_len() -> i32;
    pub(crate) fn blip_basic_source_copy(ptr: *mut u8, capacity: i32) -> i32;
    pub(crate) fn blip_basic_ready();
    fn blip_basic_file_available(ptr: *const u8, len: i32) -> i32;
    pub(crate) fn blip_basic_program_loaded();
    pub(crate) fn blip_basic_runtime_error(ptr: *const u8, len: i32);
    pub(crate) fn blip_basic_sound(frequency: f32, seconds: f32, volume: f32);
    pub(crate) fn blip_basic_audio_event();
    fn blip_basic_file_count() -> i32;
    fn blip_basic_file_name_len(index: i32) -> i32;
    fn blip_basic_file_name_copy(index: i32, ptr: *mut u8, capacity: i32) -> i32;
    fn blip_basic_file_bytes_len(index: i32) -> i32;
    fn blip_basic_file_bytes_copy(index: i32, ptr: *mut u8, capacity: i32) -> i32;
}

pub(crate) fn host_source() -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        let len = blip_basic_source_len();
        if len <= 0 {
            return None;
        }
        let mut bytes = vec![0; len as usize];
        let copied = blip_basic_source_copy(bytes.as_mut_ptr(), len);
        bytes.truncate(copied.max(0) as usize);
        String::from_utf8(bytes).ok()
    }
    // BLIP_BASIC_TAPE names a tape to run natively, for playtests.
    #[cfg(not(target_arch = "wasm32"))]
    {
        match std::env::var("BLIP_BASIC_TAPE") {
            Ok(path) => std::fs::read_to_string(&path).ok(),
            Err(_) => Some(include_str!("../../../web/TAPE/BOUNCER.BLIP").to_string()),
        }
    }
}

/// The user's pictures and sounds from /USER, which live in the browser.
#[cfg(target_arch = "wasm32")]
pub(crate) fn host_user_files() -> Vec<(String, Vec<u8>)> {
    unsafe {
        (0..blip_basic_file_count().clamp(0, 512))
            .filter_map(|index| {
                let mut name = vec![0; blip_basic_file_name_len(index).clamp(0, 4096) as usize];
                let mut bytes = vec![0; blip_basic_file_bytes_len(index).clamp(0, 16 << 20) as usize];
                let n = blip_basic_file_name_copy(index, name.as_mut_ptr(), name.len() as i32).max(0);
                let b = blip_basic_file_bytes_copy(index, bytes.as_mut_ptr(), bytes.len() as i32).max(0);
                name.truncate(n as usize);
                bytes.truncate(b as usize);
                Some((String::from_utf8(name).ok()?.to_ascii_uppercase(), bytes))
            })
            .collect()
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn host_user_files() -> Vec<(String, Vec<u8>)> {
    Vec::new()
}

/// Where a /TAPE file is: beside the runtime page on the web, in the repository's
/// web folder natively.
pub(crate) fn tape_url(path: &str) -> String {
    let path = path.trim_start_matches('/');
    if cfg!(target_arch = "wasm32") {
        path.to_string()
    } else {
        format!("{}/../../web/{path}", env!("CARGO_MANIFEST_DIR"))
    }
}

pub(crate) fn audio_event() {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        blip_basic_audio_event();
    }
}

/// Cached textures still obey the browser's current filesystem mount.
pub(crate) fn file_available(path: &str) -> bool {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        blip_basic_file_available(path.as_ptr(), path.len() as i32) != 0
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = path;
        true
    }
}
