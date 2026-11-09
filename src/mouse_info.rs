//! Détection des souris branchées. Windows ne connaît pas leur DPI (il est stocké dans la souris),
//! mais il donne leur nom et leur fabricant, ce qui permet d'indiquer où le vérifier.

pub struct Brand {
    pub name: &'static str,
    pub advice: &'static str,
}

const BRANDS: &[(u16, Brand)] = &[
    (0x046D, Brand { name: "Logitech", advice: "Ouvrez Logitech G HUB : le DPI actif y est affiché." }),
    (0x1532, Brand { name: "Razer", advice: "Ouvrez Razer Synapse : le DPI actif y est affiché." }),
    (0x1038, Brand { name: "SteelSeries", advice: "Ouvrez SteelSeries GG (Engine) : le DPI y est affiché." }),
    (0x1B1C, Brand { name: "Corsair", advice: "Ouvrez Corsair iCUE : le DPI y est affiché." }),
    (0x0951, Brand { name: "HyperX", advice: "Ouvrez HyperX NGENUITY : le DPI y est affiché." }),
    (0x03F0, Brand { name: "HyperX / HP", advice: "Ouvrez HyperX NGENUITY : le DPI y est affiché." }),
    (0x1E7D, Brand { name: "Roccat", advice: "Ouvrez Roccat / Turtle Beach Swarm : le DPI y est affiché." }),
    (0x1AF3, Brand { name: "Zowie", advice: "Pas de logiciel : le DPI se règle avec le bouton sous la souris (400 / 800 / 1600 / 3200)." }),
    (0x3367, Brand { name: "Endgame Gear", advice: "Ouvrez le logiciel Endgame Gear : le DPI y est affiché." }),
    (0x258A, Brand { name: "Glorious ou générique", advice: "Ouvrez le logiciel du fabricant (ex. Glorious CORE) pour voir le DPI." }),
    (0x045E, Brand { name: "Microsoft", advice: "Ouvrez Microsoft Souris et clavier Center." }),
];

pub struct MouseInfo {
    pub name: String,
    pub brand: Option<&'static Brand>,
}

fn brand_for(vendor_id: u16) -> Option<&'static Brand> {
    BRANDS.iter().find(|(vid, _)| *vid == vendor_id).map(|(_, brand)| brand)
}

/// `\\?\HID#VID_046D&PID_C08B&MI_00#...` → 0x046D
fn vendor_id(device_path: &str) -> Option<u16> {
    let upper = device_path.to_ascii_uppercase();
    let start = upper.find("VID_")? + 4;
    u16::from_str_radix(upper.get(start..start + 4)?, 16).ok()
}

#[cfg(windows)]
pub fn detect() -> Vec<MouseInfo> {
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Devices::HumanInterfaceDevice::{HidD_GetManufacturerString, HidD_GetProductString};
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING};
    use windows_sys::Win32::UI::Input::{
        GetRawInputDeviceInfoW, GetRawInputDeviceList, RAWINPUTDEVICELIST, RIDI_DEVICENAME, RIM_TYPEMOUSE,
    };

    fn wide_to_string(buf: &[u16]) -> String {
        let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        String::from_utf16_lossy(&buf[..end]).trim().to_owned()
    }

    let entry_size = size_of::<RAWINPUTDEVICELIST>() as u32;
    let mut count = 0u32;
    let mut devices: Vec<RAWINPUTDEVICELIST>;
    // SAFETY: appels Win32 avec des tampons dimensionnés selon les tailles renvoyées par l'API.
    unsafe {
        if GetRawInputDeviceList(null_mut(), &mut count, entry_size) == u32::MAX {
            return Vec::new();
        }
        devices = vec![std::mem::zeroed(); count as usize];
        let found = GetRawInputDeviceList(devices.as_mut_ptr(), &mut count, entry_size);
        if found == u32::MAX {
            return Vec::new();
        }
        devices.truncate(found as usize);
    }

    let mut mice: Vec<MouseInfo> = Vec::new();
    for device in devices.iter().filter(|d| d.dwType == RIM_TYPEMOUSE) {
        let path = unsafe {
            let mut len = 0u32;
            GetRawInputDeviceInfoW(device.hDevice, RIDI_DEVICENAME, null_mut(), &mut len);
            let mut buf = vec![0u16; len as usize + 1];
            if GetRawInputDeviceInfoW(device.hDevice, RIDI_DEVICENAME, buf.as_mut_ptr().cast(), &mut len) == u32::MAX {
                continue;
            }
            buf
        };
        let path_text = wide_to_string(&path);
        if !path_text.to_ascii_uppercase().contains("HID#") {
            continue;
        }
        let brand = vendor_id(&path_text).and_then(brand_for);

        let (manufacturer, product) = unsafe {
            let handle = CreateFileW(path.as_ptr(), 0, FILE_SHARE_READ | FILE_SHARE_WRITE, null(), OPEN_EXISTING, 0, null_mut());
            if handle == INVALID_HANDLE_VALUE {
                (String::new(), String::new())
            } else {
                let mut manufacturer = [0u16; 128];
                let mut product = [0u16; 128];
                let bytes = size_of_val(&product) as u32;
                let manufacturer = if HidD_GetManufacturerString(handle, manufacturer.as_mut_ptr().cast(), bytes) {
                    wide_to_string(&manufacturer)
                } else {
                    String::new()
                };
                let product = if HidD_GetProductString(handle, product.as_mut_ptr().cast(), bytes) {
                    wide_to_string(&product)
                } else {
                    String::new()
                };
                CloseHandle(handle);
                (manufacturer, product)
            }
        };

        let name = match (product.is_empty(), manufacturer.is_empty(), brand) {
            (false, false, _) if !product.to_lowercase().contains(&manufacturer.to_lowercase()) => {
                format!("{manufacturer} {product}")
            }
            (false, _, _) => product,
            (true, false, _) => format!("Souris {manufacturer}"),
            (true, true, Some(brand)) => format!("Souris {}", brand.name),
            (true, true, None) => continue,
        };
        if !mice.iter().any(|m| m.name == name) {
            mice.push(MouseInfo { name, brand });
        }
    }
    mice
}

#[cfg(not(windows))]
pub fn detect() -> Vec<MouseInfo> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_vendor_id() {
        assert_eq!(vendor_id(r"\\?\HID#VID_046D&PID_C08B&MI_00#7&1a2b#{378de44c}"), Some(0x046D));
        assert_eq!(vendor_id(r"\\?\Root#RDP_MOU#0000"), None);
    }

    #[test]
    fn detects_connected_mice() {
        for mouse in detect() {
            println!("{} -> {}", mouse.name, mouse.brand.map_or("marque inconnue", |b| b.name));
        }
    }
}
