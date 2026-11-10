//! A Mac has no volume for each app, so Mimi lowers the whole output instead. It appears here
//! as a single "session", and the shared code handles saving, fading and crash recovery.

use super::Targets;
use std::ffi::c_void;

#[repr(C)]
struct PropertyAddress {
    selector: u32,
    scope: u32,
    element: u32,
}

#[link(name = "CoreAudio", kind = "framework")]
unsafe extern "C" {
    fn AudioObjectGetPropertyData(object: u32, address: *const PropertyAddress, qualifier_size: u32, qualifier: *const c_void, size: *mut u32, data: *mut c_void) -> i32;
    fn AudioObjectSetPropertyData(object: u32, address: *const PropertyAddress, qualifier_size: u32, qualifier: *const c_void, size: u32, data: *const c_void) -> i32;
}

const SYSTEM_OBJECT: u32 = 1;
const DEFAULT_OUTPUT: u32 = u32::from_be_bytes(*b"dOut");
const MAIN_VOLUME: u32 = u32::from_be_bytes(*b"vmvc");
const GLOBAL_SCOPE: u32 = u32::from_be_bytes(*b"glob");
const OUTPUT_SCOPE: u32 = u32::from_be_bytes(*b"outp");
const MAIN_ELEMENT: u32 = 0;

fn volume_address() -> PropertyAddress {
    PropertyAddress { selector: MAIN_VOLUME, scope: OUTPUT_SCOPE, element: MAIN_ELEMENT }
}

pub struct Session {
    pub name: String,
    device: u32,
}

impl Session {
    pub fn level(&self) -> Option<f32> {
        let mut level = 0f32;
        let mut size = std::mem::size_of::<f32>() as u32;
        let status = unsafe {
            AudioObjectGetPropertyData(self.device, &volume_address(), 0, std::ptr::null(), &mut size, (&raw mut level).cast())
        };
        (status == 0).then_some(level)
    }

    pub fn set_level(&self, level: f32) {
        unsafe {
            AudioObjectSetPropertyData(self.device, &volume_address(), 0, std::ptr::null(), std::mem::size_of::<f32>() as u32, (&raw const level).cast());
        }
    }
}

fn default_output() -> Option<u32> {
    let address = PropertyAddress { selector: DEFAULT_OUTPUT, scope: GLOBAL_SCOPE, element: MAIN_ELEMENT };
    let mut device = 0u32;
    let mut size = std::mem::size_of::<u32>() as u32;
    let status = unsafe { AudioObjectGetPropertyData(SYSTEM_OBJECT, &address, 0, std::ptr::null(), &mut size, (&raw mut device).cast()) };
    (status == 0 && device != 0).then_some(device)
}

pub fn sessions(targets: Targets) -> Vec<Session> {
    // Tests ask for this app only, which on a Mac means nothing, so they never move the volume.
    if matches!(targets, Targets::OnlyThisApp) {
        return Vec::new();
    }
    let Some(device) = default_output() else { return Vec::new() };
    let session = Session { name: "Mac volume".to_string(), device };
    // Some outputs have no volume of their own. Then there's nothing Mimi can lower.
    if session.level().is_some() { vec![session] } else { Vec::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tests_can_never_reach_the_real_volume() {
        assert!(sessions(Targets::OnlyThisApp).is_empty());
    }
}
