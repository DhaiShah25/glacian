use piglog::{debug, error, note, prelude::*, warning};
use std::ffi::{CStr, c_void};
use vulkanalia::vk;

pub unsafe extern "system" fn callback(
    severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    type_: vk::DebugUtilsMessageTypeFlagsEXT,
    data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _: *mut c_void,
) -> vk::Bool32 {
    let data = unsafe { *data };
    let message = unsafe { CStr::from_ptr(data.message) };

    if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::ERROR {
        error!("({:?}) {:?}", type_, message);
    } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::WARNING {
        warning!("({:?}) {:?}", type_, message);
    } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::INFO {
        note!("({:?}) {:?}", type_, message);
    } else {
        debug!("({:?}) {:?}", type_, message);
    }

    vk::FALSE
}
