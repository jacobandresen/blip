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
    pub(crate) fn blip_basic_texture_count() -> i32;
    pub(crate) fn blip_basic_texture_name_len(index: i32) -> i32;
    pub(crate) fn blip_basic_texture_name_copy(index: i32, ptr: *mut u8, capacity: i32) -> i32;
    pub(crate) fn blip_basic_texture_png_len(index: i32) -> i32;
    pub(crate) fn blip_basic_texture_png_copy(index: i32, ptr: *mut u8, capacity: i32) -> i32;
    pub(crate) fn blip_basic_export_requested() -> i32;
    pub(crate) fn blip_basic_export_game() -> i32;
    pub(crate) fn blip_basic_export_rom_asset(
        path_ptr: *const u8,
        path_len: i32,
        png_ptr: *const u8,
        png_len: i32,
    );
    pub(crate) fn blip_basic_export_done();
    pub(crate) fn blip_basic_music_count() -> i32;
    pub(crate) fn blip_basic_music_name_len(index: i32) -> i32;
    pub(crate) fn blip_basic_music_name_copy(index: i32, ptr: *mut u8, capacity: i32) -> i32;
    pub(crate) fn blip_basic_music_wav_len(index: i32) -> i32;
    pub(crate) fn blip_basic_music_wav_copy(index: i32, ptr: *mut u8, capacity: i32) -> i32;
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
    #[cfg(not(target_arch = "wasm32"))]
    {
        Some(include_str!("../../../web/TAPE/BOUNCER.BLIP").to_string())
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn host_user_textures() -> Vec<(String, Vec<u8>)> {
    unsafe {
        let count = blip_basic_texture_count().clamp(0, 512);
        (0..count)
            .filter_map(|index| {
                let name_len = blip_basic_texture_name_len(index).clamp(0, 4096) as usize;
                let png_len = blip_basic_texture_png_len(index).clamp(0, 8 * 1024 * 1024) as usize;
                if name_len == 0 || png_len == 0 {
                    return None;
                }
                let mut name = vec![0; name_len];
                let mut png = vec![0; png_len];
                let name_copied =
                    blip_basic_texture_name_copy(index, name.as_mut_ptr(), name_len as i32).max(0)
                        as usize;
                let png_copied =
                    blip_basic_texture_png_copy(index, png.as_mut_ptr(), png_len as i32).max(0)
                        as usize;
                name.truncate(name_copied);
                png.truncate(png_copied);
                Some((String::from_utf8(name).ok()?.to_ascii_uppercase(), png))
            })
            .collect()
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn host_user_textures() -> Vec<(String, Vec<u8>)> {
    Vec::new()
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn host_user_music() -> Vec<(String, Vec<u8>)> {
    unsafe {
        let count = blip_basic_music_count().clamp(0, 64);
        (0..count)
            .filter_map(|index| {
                let name_len = blip_basic_music_name_len(index).clamp(0, 4096) as usize;
                let wav_len = blip_basic_music_wav_len(index).clamp(0, 8 * 1024 * 1024) as usize;
                if name_len == 0 || wav_len < 44 {
                    return None;
                }
                let mut name = vec![0; name_len];
                let mut wav = vec![0; wav_len];
                let name_copied =
                    blip_basic_music_name_copy(index, name.as_mut_ptr(), name_len as i32).max(0)
                        as usize;
                let wav_copied = blip_basic_music_wav_copy(index, wav.as_mut_ptr(), wav_len as i32)
                    .max(0) as usize;
                name.truncate(name_copied);
                wav.truncate(wav_copied);
                Some((String::from_utf8(name).ok()?.to_ascii_uppercase(), wav))
            })
            .collect()
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn host_user_music() -> Vec<(String, Vec<u8>)> {
    Vec::new()
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
