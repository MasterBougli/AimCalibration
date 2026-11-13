//! Petits appels Windows : accélération du pointeur et presse-papiers.

/// « Améliorer la précision du pointeur » est activé quand le troisième champ de SPI_GETMOUSE est non nul.
pub fn enhance_pointer_precision() -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{SPI_GETMOUSE, SystemParametersInfoW};
        let mut mouse = [0i32; 3];
        unsafe { SystemParametersInfoW(SPI_GETMOUSE, 0, mouse.as_mut_ptr().cast(), 0) != 0 && mouse[2] != 0 }
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Copie `text` dans le presse-papiers. Le système devient propriétaire de la mémoire si l'appel réussit.
pub fn copy_text(text: &str) -> bool {
    #[cfg(windows)]
    {
        use std::iter;
        use windows_sys::Win32::Foundation::GlobalFree;
        use windows_sys::Win32::System::DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData};
        use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

        let wide: Vec<u16> = text.encode_utf16().chain(iter::once(0)).collect();
        unsafe {
            if OpenClipboard(std::ptr::null_mut()) == 0 {
                return false;
            }
            EmptyClipboard();
            let mem = GlobalAlloc(GMEM_MOVEABLE, wide.len() * size_of::<u16>());
            if mem.is_null() {
                CloseClipboard();
                return false;
            }
            let ptr = GlobalLock(mem).cast::<u16>();
            if ptr.is_null() {
                GlobalFree(mem);
                CloseClipboard();
                return false;
            }
            std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr, wide.len());
            GlobalUnlock(mem);
            let ok = !SetClipboardData(13, mem).is_null();
            if !ok {
                GlobalFree(mem);
            }
            CloseClipboard();
            ok
        }
    }
    #[cfg(not(windows))]
    {
        let _ = text;
        false
    }
}
