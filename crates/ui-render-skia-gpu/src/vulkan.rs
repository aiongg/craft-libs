//! Ganesh on Vulkan (Linux, Windows, Android) with its own instance, device and queue (ash).
//!
//! The renderer does not share wgpu's device: wgpu tracks image layouts and swapchain acquire
//! semaphores itself, and Skia submitting on wgpu's raw queue would bypass both (a surface texture
//! wgpu never saw written is cleared at present, and its acquire semaphore is never waited on).
//!
//! A window gets its own swapchain. Presenting copies the kept canvas onto the acquired image with
//! Skia, transitions it to the present layout in the same flush, and submits with a CPU wait
//! before `vkQueuePresentKHR`, as skia-safe's Vulkan example does; a window frame therefore waits
//! for its GPU work. The bench runs offscreen and does not.

use std::ffi::{CStr, c_char, c_void};
use std::mem::ManuallyDrop;
use std::ptr;
use std::sync::Arc;

use ash::vk::{self, Handle};
use craft_ui_rt::SizeU;
use skia_safe::gpu::vk as skvk;
use skia_safe::gpu::{self, DirectContext, FlushInfo, SurfaceOrigin, SyncCpu};
use skia_safe::{ColorType, Surface};
use web_time::Instant;

use super::{Keep, Submitted, WindowRef, blit, render_target};

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

/// Two command buffers that write a timestamp each, submitted around Skia's submission.
struct Timer {
    pool: vk::QueryPool,
    commands: vk::CommandPool,
    begin: vk::CommandBuffer,
    end: vk::CommandBuffer,
    done: vk::Fence,
    /// Nanoseconds per timestamp tick.
    period: f64,
    /// The bits a timestamp of this queue holds.
    mask: u64,
}

impl Timer {
    unsafe fn new(device: &ash::Device, family: u32, valid_bits: u32, period: f32) -> ash::prelude::VkResult<Timer> {
        unsafe {
            let pool = device.create_query_pool(&vk::QueryPoolCreateInfo::default().query_type(vk::QueryType::TIMESTAMP).query_count(2), None)?;
            let commands = device.create_command_pool(&vk::CommandPoolCreateInfo::default().queue_family_index(family), None)?;
            let buffers = device.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::default().command_pool(commands).level(vk::CommandBufferLevel::PRIMARY).command_buffer_count(2),
            )?;
            let (begin, end) = (buffers[0], buffers[1]);
            // Recorded once; each submission waits for the last one's fence first.
            device.begin_command_buffer(begin, &vk::CommandBufferBeginInfo::default())?;
            device.cmd_reset_query_pool(begin, pool, 0, 2);
            device.cmd_write_timestamp(begin, vk::PipelineStageFlags::TOP_OF_PIPE, pool, 0);
            device.end_command_buffer(begin)?;
            device.begin_command_buffer(end, &vk::CommandBufferBeginInfo::default())?;
            device.cmd_write_timestamp(end, vk::PipelineStageFlags::BOTTOM_OF_PIPE, pool, 1);
            device.end_command_buffer(end)?;
            let done = device.create_fence(&vk::FenceCreateInfo::default(), None)?;
            let mask = if valid_bits >= 64 { u64::MAX } else { (1u64 << valid_bits) - 1 };
            Ok(Timer { pool, commands, begin, end, done, period: f64::from(period), mask })
        }
    }

    fn submit(&self, device: &ash::Device, queue: vk::Queue, buffer: vk::CommandBuffer, fence: vk::Fence) -> bool {
        let buffers = [buffer];
        let submit = [vk::SubmitInfo::default().command_buffers(&buffers)];
        unsafe { device.queue_submit(queue, &submit, fence) }.map_err(|e| log::error!("timestamp submit: {e}")).is_ok()
    }

    /// Waits for the end timestamp and returns the time between the two, in milliseconds.
    fn read(&self, device: &ash::Device) -> Option<f64> {
        let mut stamps = [0u64; 2];
        unsafe {
            device.wait_for_fences(&[self.done], true, u64::MAX).ok()?;
            device.reset_fences(&[self.done]).ok()?;
            device.get_query_pool_results(self.pool, 0, &mut stamps, vk::QueryResultFlags::TYPE_64 | vk::QueryResultFlags::WAIT).ok()?;
        }
        let ticks = stamps[1].wrapping_sub(stamps[0]) & self.mask;
        Some(ticks as f64 * self.period / 1e6)
    }

    unsafe fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_fence(self.done, None);
            device.destroy_command_pool(self.commands, None);
            device.destroy_query_pool(self.pool, None);
        }
    }
}

pub struct Gpu {
    context: ManuallyDrop<DirectContext>,
    timer: Option<Timer>,
    queue: vk::Queue,
    family: u32,
    physical: vk::PhysicalDevice,
    /// The window surface made to pick the device, until the first presenter takes it.
    first_surface: Option<(Keep, vk::SurfaceKHR)>,
    /// Present only when the instance and device were made for a window.
    surface_fn: Option<ash::khr::surface::Instance>,
    swapchain_fn: Option<ash::khr::swapchain::Device>,
    device: ash::Device,
    instance: ash::Instance,
    entry: ash::Entry,
}

impl Gpu {
    /// The device and Skia's context. With a window, the instance gets the platform's surface
    /// extensions, the device the swapchain extension, and the queue family must present to it.
    pub fn new(window: Option<&WindowRef>) -> Result<Gpu, String> {
        let entry = unsafe { ash::Entry::load() }.map_err(|e| format!("no Vulkan loader: {e}"))?;
        let display = match window {
            Some(w) => Some(w.display),
            None => None,
        };
        let instance_ext: Vec<*const c_char> = match display {
            Some(d) => ash_window::enumerate_required_extensions(d).map_err(|e| format!("surface extensions: {e}"))?.to_vec(),
            None => Vec::new(),
        };
        // Skia needs Vulkan 1.1 and is told to stay within it.
        let app = vk::ApplicationInfo::default().api_version(vk::API_VERSION_1_1);
        let info = vk::InstanceCreateInfo::default().application_info(&app).enabled_extension_names(&instance_ext);
        let instance = unsafe { entry.create_instance(&info, None) }.map_err(|e| format!("Vulkan instance: {e}"))?;
        let surface_fn = display.map(|_| ash::khr::surface::Instance::new(&entry, &instance));
        let first_surface = match (window, display, &surface_fn) {
            (Some(w), Some(d), Some(_)) => {
                let surface =
                    unsafe { ash_window::create_surface(&entry, &instance, d, w.window, None) }.map_err(|e| format!("Vulkan surface: {e}"))?;
                Some((w.keep.clone(), surface))
            }
            _ => None,
        };
        let (physical, family, props, timestamp_bits) = Self::pick(&instance, surface_fn.as_ref(), first_surface.as_ref().map(|(_, s)| *s))
            .ok_or("no Vulkan 1.1 device with a graphics queue (that presents to the window)")?;
        log::info!("skia-gpu: Vulkan device {:?} ({:?})", props.device_name_as_c_str().unwrap_or_default(), props.device_type);

        let priorities = [1.0f32];
        let queues = [vk::DeviceQueueCreateInfo::default().queue_family_index(family).queue_priorities(&priorities)];
        let device_ext: Vec<*const c_char> = if window.is_some() { vec![ash::khr::swapchain::NAME.as_ptr()] } else { Vec::new() };
        let device = unsafe {
            instance.create_device(physical, &vk::DeviceCreateInfo::default().queue_create_infos(&queues).enabled_extension_names(&device_ext), None)
        }
        .map_err(|e| format!("Vulkan device: {e}"))?;
        let queue = unsafe { device.get_device_queue(family, 0) };
        let swapchain_fn = window.map(|_| ash::khr::swapchain::Device::new(&instance, &device));

        let context = Self::skia(&entry, &instance, physical, &device, queue, family, &instance_ext, &device_ext)
            .ok_or("Skia could not make a Vulkan context")?;
        let timer = (timestamp_bits > 0)
            .then(|| unsafe { Timer::new(&device, family, timestamp_bits, props.limits.timestamp_period) })
            .and_then(|t| t.map_err(|e| log::error!("timestamp queries: {e}")).ok());
        Ok(Gpu {
            context: ManuallyDrop::new(context),
            timer,
            queue,
            family,
            physical,
            first_surface,
            surface_fn,
            swapchain_fn,
            device,
            instance,
            entry,
        })
    }

    /// The best-ranked device (discrete, integrated, virtual, CPU) with Vulkan 1.1 and a graphics
    /// queue family that presents to `surface`; with the family's timestamp bits (0 = none).
    fn pick(
        instance: &ash::Instance,
        surface_fn: Option<&ash::khr::surface::Instance>,
        surface: Option<vk::SurfaceKHR>,
    ) -> Option<(vk::PhysicalDevice, u32, vk::PhysicalDeviceProperties, u32)> {
        let devices = unsafe { instance.enumerate_physical_devices() }.ok()?;
        let rank = |t: vk::PhysicalDeviceType| match t {
            vk::PhysicalDeviceType::DISCRETE_GPU => 0,
            vk::PhysicalDeviceType::INTEGRATED_GPU => 1,
            vk::PhysicalDeviceType::VIRTUAL_GPU => 2,
            vk::PhysicalDeviceType::CPU => 3,
            _ => 4,
        };
        devices
            .into_iter()
            .filter_map(|pd| {
                let props = unsafe { instance.get_physical_device_properties(pd) };
                if props.api_version < vk::API_VERSION_1_1 {
                    return None;
                }
                let families = unsafe { instance.get_physical_device_queue_family_properties(pd) };
                let (index, family) = families.iter().enumerate().find(|(i, f)| {
                    f.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                        && match (surface_fn, surface) {
                            (Some(sf), Some(s)) => unsafe { sf.get_physical_device_surface_support(pd, *i as u32, s) }.unwrap_or(false),
                            _ => true,
                        }
                })?;
                let bits = if props.limits.timestamp_compute_and_graphics == vk::TRUE { family.timestamp_valid_bits } else { 0 };
                Some((pd, index as u32, props, bits))
            })
            .min_by_key(|(_, _, props, _)| rank(props.device_type))
    }

    #[allow(clippy::too_many_arguments)]
    fn skia(
        entry: &ash::Entry,
        instance: &ash::Instance,
        physical: vk::PhysicalDevice,
        device: &ash::Device,
        queue: vk::Queue,
        family: u32,
        instance_ext: &[*const c_char],
        device_ext: &[*const c_char],
    ) -> Option<DirectContext> {
        let get_proc = |of: skvk::GetProcOf| -> *const c_void {
            let f = unsafe {
                match of {
                    skvk::GetProcOf::Instance(i, name) => entry.get_instance_proc_addr(vk::Instance::from_raw(i as u64), name),
                    skvk::GetProcOf::Device(d, name) => instance.get_device_proc_addr(vk::Device::from_raw(d as u64), name),
                }
            };
            f.map_or(ptr::null(), |f| f as *const c_void)
        };
        let names =
            |list: &[*const c_char]| -> Vec<String> { list.iter().map(|&p| unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()).collect() };
        let (instance_ext, device_ext) = (names(instance_ext), names(device_ext));
        let instance_ext: Vec<&str> = instance_ext.iter().map(String::as_str).collect();
        let device_ext: Vec<&str> = device_ext.iter().map(String::as_str).collect();
        let backend = unsafe {
            skvk::BackendContext::new_builder(
                instance.handle().as_raw() as _,
                physical.as_raw() as _,
                device.handle().as_raw() as _,
                (queue.as_raw() as _, family as usize),
                &get_proc,
                Some(skvk::Version::new(1, 1, 0)),
            )
            .with_extensions(&instance_ext, &device_ext)
            .build()
        };
        gpu::direct_contexts::make_vulkan(&backend, None)
    }

    pub fn surface(&mut self, size: SizeU) -> Option<Surface> {
        render_target(&mut self.context, size)
    }

    pub fn presenter(&mut self, window: WindowRef) -> Option<Presenter> {
        let (Some(surface_fn), Some(swapchain_fn)) = (self.surface_fn.clone(), self.swapchain_fn.clone()) else {
            log::error!("skia-gpu: this device was made without a window and cannot present");
            return None;
        };
        let surface = match self.first_surface.take() {
            Some((w, s)) if Arc::ptr_eq(&w, &window.keep) => s,
            other => {
                if let Some((_, s)) = other {
                    unsafe { surface_fn.destroy_surface(s, None) };
                }
                let s = unsafe { ash_window::create_surface(&self.entry, &self.instance, window.display, window.window, None) }
                    .map_err(|e| log::error!("Vulkan surface: {e}"))
                    .ok()?;
                let supported = unsafe { surface_fn.get_physical_device_surface_support(self.physical, self.family, s) }.unwrap_or(false);
                if !supported {
                    log::error!("skia-gpu: the queue family cannot present to this window");
                    unsafe { surface_fn.destroy_surface(s, None) };
                    return None;
                }
                s
            }
        };
        let acquired = unsafe { self.device.create_fence(&vk::FenceCreateInfo::default(), None) }.map_err(|e| log::error!("fence: {e}")).ok()?;
        Some(Presenter {
            window: window.keep,
            surface,
            swapchain: vk::SwapchainKHR::null(),
            images: Vec::new(),
            format: (skvk::Format::B8G8R8A8_UNORM, ColorType::BGRA8888),
            size: SizeU::default(),
            stale: true,
            acquired,
            physical: self.physical,
            device: self.device.clone(),
            surface_fn,
            swapchain_fn,
        })
    }

    pub fn submit(&mut self, canvas: &mut Surface, presenter: Option<&mut Presenter>, timing: bool) -> Submitted {
        let t = Instant::now();
        if let Some(p) = presenter {
            p.present(&mut self.context, self.queue, self.family, canvas);
            return Submitted { submit_ms: ms(t), gpu_ms: None };
        }
        if let Err(e) = self.context.flush(&FlushInfo::default()) {
            log::error!("skia flush failed: {e:?}");
        }
        let mut submit_ms = ms(t);
        let timer = if timing { self.timer.as_ref() } else { None };
        let bracketed = timer.is_some_and(|tm| tm.submit(&self.device, self.queue, tm.begin, vk::Fence::null()));
        let t = Instant::now();
        if !self.context.submit(SyncCpu::No) {
            log::error!("skia submit failed");
        }
        submit_ms += ms(t);
        let gpu_ms = match timer {
            Some(tm) if bracketed && tm.submit(&self.device, self.queue, tm.end, tm.done) => tm.read(&self.device),
            _ => None,
        };
        Submitted { submit_ms, gpu_ms }
    }
}

impl Drop for Gpu {
    fn drop(&mut self) {
        unsafe {
            ManuallyDrop::drop(&mut self.context);
            let _ = self.device.device_wait_idle();
            if let Some(t) = self.timer.take() {
                t.destroy(&self.device);
            }
            if let (Some((_, s)), Some(sf)) = (self.first_surface.take(), &self.surface_fn) {
                sf.destroy_surface(s, None);
            }
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}

/// A window's swapchain.
pub struct Presenter {
    window: Keep,
    surface: vk::SurfaceKHR,
    swapchain: vk::SwapchainKHR,
    images: Vec<vk::Image>,
    format: (skvk::Format, ColorType),
    /// The canvas size the swapchain should have.
    size: SizeU,
    /// The swapchain is missing, suboptimal, out of date or the wrong size.
    stale: bool,
    acquired: vk::Fence,
    physical: vk::PhysicalDevice,
    device: ash::Device,
    surface_fn: ash::khr::surface::Instance,
    swapchain_fn: ash::khr::swapchain::Device,
}

impl Presenter {
    /// What the window handles borrow from; the window's identity.
    pub fn keep(&self) -> &Keep {
        &self.window
    }

    pub fn resize(&mut self, size: SizeU) {
        if size != self.size {
            self.size = size;
            self.stale = true;
        }
    }

    fn recreate(&mut self) -> bool {
        let caps = match unsafe { self.surface_fn.get_physical_device_surface_capabilities(self.physical, self.surface) } {
            Ok(c) => c,
            Err(e) => {
                log::error!("surface capabilities: {e}");
                return false;
            }
        };
        let extent = if caps.current_extent.width != u32::MAX {
            caps.current_extent
        } else {
            vk::Extent2D {
                width: self.size.width.clamp(caps.min_image_extent.width, caps.max_image_extent.width),
                height: self.size.height.clamp(caps.min_image_extent.height, caps.max_image_extent.height),
            }
        };
        if extent.width == 0 || extent.height == 0 {
            return false;
        }
        let formats = unsafe { self.surface_fn.get_physical_device_surface_formats(self.physical, self.surface) }.unwrap_or_default();
        let Some((chosen, format)) = formats.iter().find_map(|f| match f.format {
            vk::Format::B8G8R8A8_UNORM => Some((*f, (skvk::Format::B8G8R8A8_UNORM, ColorType::BGRA8888))),
            vk::Format::R8G8B8A8_UNORM => Some((*f, (skvk::Format::R8G8B8A8_UNORM, ColorType::RGBA8888))),
            _ => None,
        }) else {
            log::error!("skia-gpu: the window offers no BGRA8 or RGBA8 UNORM format: {formats:?}");
            return false;
        };
        let composite = [
            vk::CompositeAlphaFlagsKHR::OPAQUE,
            vk::CompositeAlphaFlagsKHR::INHERIT,
            vk::CompositeAlphaFlagsKHR::PRE_MULTIPLIED,
            vk::CompositeAlphaFlagsKHR::POST_MULTIPLIED,
        ]
        .into_iter()
        .find(|c| caps.supported_composite_alpha.contains(*c))
        .unwrap_or(vk::CompositeAlphaFlagsKHR::OPAQUE);
        let max = if caps.max_image_count == 0 { u32::MAX } else { caps.max_image_count };
        let info = vk::SwapchainCreateInfoKHR::default()
            .surface(self.surface)
            .min_image_count((caps.min_image_count + 1).min(max))
            .image_format(chosen.format)
            .image_color_space(chosen.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(caps.current_transform)
            .composite_alpha(composite)
            .present_mode(vk::PresentModeKHR::FIFO)
            .clipped(true)
            .old_swapchain(self.swapchain);
        unsafe {
            let _ = self.device.device_wait_idle();
            let swapchain = match self.swapchain_fn.create_swapchain(&info, None) {
                Ok(s) => s,
                Err(e) => {
                    log::error!("swapchain: {e}");
                    return false;
                }
            };
            if self.swapchain != vk::SwapchainKHR::null() {
                self.swapchain_fn.destroy_swapchain(self.swapchain, None);
            }
            self.swapchain = swapchain;
            self.images = self.swapchain_fn.get_swapchain_images(swapchain).unwrap_or_default();
        }
        self.format = format;
        self.stale = false;
        true
    }

    fn present(&mut self, context: &mut DirectContext, queue: vk::Queue, family: u32, canvas: &mut Surface) {
        if self.stale && !self.recreate() {
            return;
        }
        let index = match unsafe { self.swapchain_fn.acquire_next_image(self.swapchain, u64::MAX, vk::Semaphore::null(), self.acquired) } {
            Ok((index, suboptimal)) => {
                self.stale |= suboptimal;
                index
            }
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                self.stale = true;
                return;
            }
            Err(e) => {
                log::error!("acquire: {e}");
                return;
            }
        };
        unsafe {
            let _ = self.device.wait_for_fences(&[self.acquired], true, u64::MAX);
            let _ = self.device.reset_fences(&[self.acquired]);
        }
        let Some(&image) = self.images.get(index as usize) else { return };
        let (format, color_type) = self.format;
        let (w, h) = (canvas.width(), canvas.height());
        let info = unsafe {
            skvk::ImageInfo::new(
                image.as_raw() as _,
                skvk::Alloc::default(),
                skvk::ImageTiling::OPTIMAL,
                skvk::ImageLayout::UNDEFINED,
                format,
                1,
                None,
                None,
                None,
                None,
            )
        };
        let target = gpu::backend_render_targets::make_vk((w, h), &info);
        let Some(mut onto) = gpu::surfaces::wrap_backend_render_target(context, &target, SurfaceOrigin::TopLeft, color_type, None, None) else {
            log::error!("skia could not wrap the swapchain image");
            return;
        };
        blit(canvas, &mut onto);
        let present = skvk::mutable_texture_states::new_vulkan(skvk::ImageLayout::PRESENT_SRC_KHR, family);
        if let Err(e) = context.flush_surface_with_texture_state(&mut onto, &FlushInfo::default(), Some(&present)) {
            log::error!("skia flush failed, frame dropped: {e:?}");
            return;
        }
        // The CPU waits for the GPU, so the present needs no semaphore.
        context.submit(SyncCpu::Yes);
        drop(onto);
        let swapchains = [self.swapchain];
        let indices = [index];
        let info = vk::PresentInfoKHR::default().swapchains(&swapchains).image_indices(&indices);
        match unsafe { self.swapchain_fn.queue_present(queue, &info) } {
            Ok(suboptimal) => self.stale |= suboptimal,
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => self.stale = true,
            Err(e) => log::error!("present: {e}"),
        }
    }
}

impl Drop for Presenter {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();
            if self.swapchain != vk::SwapchainKHR::null() {
                self.swapchain_fn.destroy_swapchain(self.swapchain, None);
            }
            self.surface_fn.destroy_surface(self.surface, None);
            self.device.destroy_fence(self.acquired, None);
        }
    }
}
