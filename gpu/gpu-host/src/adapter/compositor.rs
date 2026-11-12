//! The GPU the Wayland compositor draws with ([`super::adapter_order`] ranks it before the displays).
//!
//! A window drawn on another GPU reaches the compositor as that GPU's buffers, and the compositor
//! may not show them: it can refuse them and end the window's connection (#502), or take them and
//! show nothing, with no error on either side, so neither the restart on a failed adapter nor
//! the first-frame watch notices. The second happened on a hybrid laptop whose Hyprland runs on
//! the NVIDIA GPU (`AQ_DRM_DEVICES`) while the built-in panel hangs off the Intel one: the panel
//! rule picked Intel and the window stayed empty. Which GPU drives a display is a guess at the
//! compositor's GPU; the compositor states it, as the `main_device` of its linux-dmabuf feedback
//! (protocol version 4): the device it imports client buffers on.

#[cfg(target_os = "linux")]
use wayland_client::globals::{GlobalListContents, registry_queue_init};
#[cfg(target_os = "linux")]
use wayland_client::protocol::wl_registry::{self, WlRegistry};
#[cfg(target_os = "linux")]
use wayland_client::{Connection, Dispatch, QueueHandle};
#[cfg(target_os = "linux")]
use wayland_protocols::wp::linux_dmabuf::zv1::client::zwp_linux_dmabuf_feedback_v1::{self, ZwpLinuxDmabufFeedbackV1};
#[cfg(target_os = "linux")]
use wayland_protocols::wp::linux_dmabuf::zv1::client::zwp_linux_dmabuf_v1::{self, ZwpLinuxDmabufV1};

/// PCI `(vendor, device)` ids of the GPU the Wayland compositor draws with. `None` outside a
/// Wayland session, when the compositor doesn't say (linux-dmabuf older than version 4), when its
/// device isn't a PCI GPU, and on other systems.
#[cfg(target_os = "linux")]
pub fn compositor_gpu() -> Option<(u32, u32)> {
    let device = match main_device() {
        Ok(device) => device,
        Err(why) => {
            log::debug!("the compositor's GPU is unknown: {why}");
            return None;
        }
    };
    let pci = pci_ids(std::path::Path::new("/sys/dev/char"), device);
    match pci {
        Some((vendor, id)) => log::info!("the compositor draws with {vendor:04x}:{id:04x}"),
        None => log::debug!("the compositor's device {:?} has no PCI ids", major_minor(device)),
    }
    pci
}

/// The GPU the Wayland compositor draws with: none outside Linux.
#[cfg(not(target_os = "linux"))]
pub fn compositor_gpu() -> Option<(u32, u32)> {
    None
}

/// The `main_device` of the compositor's default linux-dmabuf feedback, asked on a connection of
/// its own that is closed again before the window opens.
#[cfg(target_os = "linux")]
fn main_device() -> Result<u64, String> {
    let conn = Connection::connect_to_env().map_err(|e| format!("no Wayland compositor: {e}"))?;
    let (globals, mut queue) = registry_queue_init::<Feedback>(&conn).map_err(|e| e.to_string())?;
    let qh = queue.handle();
    // Version 4 brought the feedback; older compositors only list formats.
    let dmabuf: ZwpLinuxDmabufV1 = globals.bind(&qh, 4..=4, ()).map_err(|e| format!("linux-dmabuf version 4: {e}"))?;
    let _feedback = dmabuf.get_default_feedback(&qh, ());
    let mut state = Feedback::default();
    // The compositor sends the whole feedback, ending with `done`, in reply; the bound only keeps
    // a compositor that never ends it from holding up the start.
    for _ in 0..4 {
        if state.done {
            break;
        }
        queue.roundtrip(&mut state).map_err(|e| e.to_string())?;
    }
    // Dropping the connection releases the objects.
    let device = state.main_device.ok_or("the compositor named no main device")?;
    // A `dev_t`, in this machine's byte order.
    let device: [u8; 8] = device.try_into().map_err(|d: Vec<u8>| format!("a {}-byte device number", d.len()))?;
    Ok(u64::from_ne_bytes(device))
}

/// What [`main_device`] collects from the feedback events.
#[cfg(target_os = "linux")]
#[derive(Default)]
struct Feedback {
    main_device: Option<Vec<u8>>,
    done: bool,
}

#[cfg(target_os = "linux")]
impl Dispatch<WlRegistry, GlobalListContents> for Feedback {
    fn event(_: &mut Self, _: &WlRegistry, _: wl_registry::Event, _: &GlobalListContents, _: &Connection, _: &QueueHandle<Self>) {}
}

#[cfg(target_os = "linux")]
impl Dispatch<ZwpLinuxDmabufV1, ()> for Feedback {
    // Version 4 sends no format or modifier events: the feedback replaces them.
    fn event(_: &mut Self, _: &ZwpLinuxDmabufV1, _: zwp_linux_dmabuf_v1::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

#[cfg(target_os = "linux")]
impl Dispatch<ZwpLinuxDmabufFeedbackV1, ()> for Feedback {
    fn event(
        state: &mut Self,
        _: &ZwpLinuxDmabufFeedbackV1,
        event: zwp_linux_dmabuf_feedback_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zwp_linux_dmabuf_feedback_v1::Event::MainDevice { device } => state.main_device = Some(device),
            zwp_linux_dmabuf_feedback_v1::Event::Done => state.done = true,
            _ => {}
        }
    }
}

/// The major and minor numbers of a Linux `dev_t`, as glibc's and musl's `major()` and `minor()`
/// split it.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(super) fn major_minor(device: u64) -> (u64, u64) {
    let major = ((device & 0x0000_0000_000f_ff00) >> 8) | ((device & 0xffff_f000_0000_0000) >> 32);
    let minor = (device & 0x0000_0000_0000_00ff) | ((device & 0x0000_0fff_fff0_0000) >> 12);
    (major, minor)
}

/// The PCI ids of the GPU behind DRM device `device` (its `cardN` or `renderDN` node), from
/// `<sys_dev_char>/MAJOR:MINOR/device/{vendor,device}`.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn pci_ids(sys_dev_char: &std::path::Path, device: u64) -> Option<(u32, u32)> {
    let (major, minor) = major_minor(device);
    let gpu = sys_dev_char.join(format!("{major}:{minor}")).join("device");
    Some((super::read_sysfs_id(&gpu.join("vendor"))?, super::read_sysfs_id(&gpu.join("device"))?))
}
