use ash::vk;
use tracing::{error, info, trace, warn};

pub unsafe extern "system" fn callback(
    severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    type_: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut std::ffi::c_void,
) -> vk::Bool32 {
    use std::ffi::CStr;

    let message = unsafe { CStr::from_ptr((*p_callback_data).p_message) };

    if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::ERROR {
        error!("({:?}) {:?}", type_, message);
    } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::WARNING {
        warn!("({:?}) {:?}", type_, message);
    } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::INFO {
        info!("({:?}) {:?}", type_, message);
    } else {
        trace!("({:?}) {:?}", type_, message);
    }

    vk::FALSE
}

pub struct DebugUtils {
    instance: ash::ext::debug_utils::Instance,
    messenger: ash::vk::DebugUtilsMessengerEXT,
}

impl DebugUtils {
    pub fn new(
        instance: ash::ext::debug_utils::Instance,
        messenger: ash::vk::DebugUtilsMessengerEXT,
    ) -> Self {
        Self {
            instance,
            messenger,
        }
    }

    pub fn destroy(&mut self) {
        unsafe {
            self.instance
                .destroy_debug_utils_messenger(self.messenger, None)
        };
    }
}
