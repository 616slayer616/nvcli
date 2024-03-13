use std::default::Default;
use std::fmt::format;
use std::mem::{size_of, size_of_val};
use std::ptr::{addr_of_mut};
use crate::cli::error::Result;
use nvapi_sys_new::{make_nvapi_version, NvAPI_DISP_GetDisplayConfig, NvAPI_DISP_SetDisplayConfig, _NvAPI_Status_NVAPI_OK, NV_DISPLAYCONFIG_PATH_ADVANCED_TARGET_INFO, NV_DISPLAYCONFIG_PATH_INFO, NV_DISPLAYCONFIG_PATH_TARGET_INFO_V2, NV_DISPLAYCONFIG_SOURCE_MODE_INFO_V1, NvAPI_DISP_TryCustomDisplay, NV_CUSTOM_DISPLAY, NV_TIMING, NV_TIMINGEXT, NV_VIEWPORTF, NV_TIMING_INPUT, _NV_TIMING_OVERRIDE, NV_TIMING_OVERRIDE, _NV_TIMING_OVERRIDE_NV_TIMING_OVERRIDE_AUTO, NV_TIMING_FLAG, NvAPI_DISP_GetTiming};

use super::{general::get_status_message, rotation::Rotation, scaling::Scaling};

pub fn get_display_config() -> Result<Vec<NvDisplayConfigPathInfo>> {
    let mut path_info_count: u32 = 0;
    // Get count
    unsafe {
        let result = NvAPI_DISP_GetDisplayConfig(&mut path_info_count, std::ptr::null_mut());
        if result != 0 {
            return Err(get_status_message(&result));
        }
    }
    // Allocate path info
    let mut path_info = vec![];
    for _ in 0..path_info_count {
        path_info.push(NV_DISPLAYCONFIG_PATH_INFO {
            version: make_nvapi_version::<NV_DISPLAYCONFIG_PATH_INFO>(2),
            sourceModeInfo: Box::into_raw(
                Box::new(NV_DISPLAYCONFIG_SOURCE_MODE_INFO_V1::default()),
            ),
            ..Default::default()
        });
    }

    unsafe {
        let result = NvAPI_DISP_GetDisplayConfig(&mut path_info_count, path_info.as_mut_ptr());
        if result != 0 {
            return Err(get_status_message(&result));
        }
    }

    for info in path_info.iter_mut() {
        info.targetInfo = Box::into_raw(
            vec![
                NV_DISPLAYCONFIG_PATH_TARGET_INFO_V2 {
                    details: Box::into_raw(Box::new(NV_DISPLAYCONFIG_PATH_ADVANCED_TARGET_INFO {
                        version: make_nvapi_version::<NV_DISPLAYCONFIG_PATH_ADVANCED_TARGET_INFO>(
                            1
                        ),
                        ..Default::default()
                    })),
                    ..Default::default()
                };
                info.targetInfoCount as usize
            ]
            .into_boxed_slice(),
        ) as *mut NV_DISPLAYCONFIG_PATH_TARGET_INFO_V2;
    }

    // Get target info
    unsafe {
        let result = NvAPI_DISP_GetDisplayConfig(&mut path_info_count, path_info.as_mut_ptr());
        if result != 0 {
            return Err(get_status_message(&result));
        }
    }

    // Collect outputs
    let output: Vec<NvDisplayConfigPathInfo> = path_info
        .into_iter()
        .map(NvDisplayConfigPathInfo::from)
        .collect();
    Ok(output)
}

pub fn set_display_config(config: Vec<NvDisplayConfigPathInfo>) -> Result<()> {
    let mut config: Vec<NV_DISPLAYCONFIG_PATH_INFO> = config
        .into_iter()
        .map(NV_DISPLAYCONFIG_PATH_INFO::from)
        .collect();
    let result;
    unsafe {
        result = NvAPI_DISP_SetDisplayConfig(config.len() as u32, config.as_mut_ptr(), 0);
    }

    // Change back for cleanup
    let _config: Vec<NvDisplayConfigPathInfo> = config
        .into_iter()
        .map(NvDisplayConfigPathInfo::from)
        .collect();

    if result != _NvAPI_Status_NVAPI_OK {
        Err(get_status_message(&result))
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct NvDisplayConfigPathInfo {
    pub target_info: Vec<NvDisplayConfigPathTargetInfo>,
    pub source_mode_info: Box<NV_DISPLAYCONFIG_SOURCE_MODE_INFO_V1>,
    pub is_non_nvidia_adapter: bool,
}

#[derive(Debug, Clone)]
pub struct NvDisplayConfigPathTargetInfo {
    pub display_id: u32,
    pub details: Box<NV_DISPLAYCONFIG_PATH_ADVANCED_TARGET_INFO>,
    pub target_id: u32,
}

pub trait Output {
    fn print_short(&self);
    fn long_display(&self) -> String;
}

impl Output for NvDisplayConfigPathInfo {
    fn print_short(&self) {
        bunt::println!(
            "{[blue+bold]}\nPrimary: {}\nResolution: {}x{}\nPosition: ({},{})",
            "Source",
            if self.source_mode_info.bGDIPrimary() == 1 {
                "true"
            } else {
                "false"
            },
            self.source_mode_info.resolution.width,
            self.source_mode_info.resolution.height,
            self.source_mode_info.position.x,
            self.source_mode_info.position.y,
        );
        for (i, target) in self.target_info.iter().enumerate() {
            bunt::println!(
                "{[green+bold]} {[green+bold]}\nID: {}\nRefresh rate: {} Hz\nScaling: {}\nRotation: {}",
                "Target",
                (i + 1).to_string(),
                target.display_id,
                target.details.refreshRate1K / 1000,
                Scaling::from(target.details.scaling),
                Rotation(target.details.rotation)
            );
        }
    }

    fn long_display(&self) -> String {
        todo!()
    }
}

impl From<NvDisplayConfigPathInfo> for NV_DISPLAYCONFIG_PATH_INFO {
    fn from(e: NvDisplayConfigPathInfo) -> Self {
        let mut targets: Vec<NV_DISPLAYCONFIG_PATH_TARGET_INFO_V2> = vec![];
        for target in e.target_info {
            targets.push(NV_DISPLAYCONFIG_PATH_TARGET_INFO_V2 {
                displayId: target.display_id,
                details: Box::into_raw(target.details),
                targetId: target.target_id,
            });
        }
        NV_DISPLAYCONFIG_PATH_INFO {
            version: make_nvapi_version::<NV_DISPLAYCONFIG_PATH_INFO>(2),
            sourceModeInfo: Box::into_raw(e.source_mode_info),
            targetInfoCount: targets.len() as u32,
            targetInfo: Box::into_raw(targets.into_boxed_slice())
                as *mut NV_DISPLAYCONFIG_PATH_TARGET_INFO_V2,
            ..Default::default()
        }
    }
}

impl From<NV_DISPLAYCONFIG_PATH_INFO> for NvDisplayConfigPathInfo {
    fn from(info: NV_DISPLAYCONFIG_PATH_INFO) -> Self {
        let mut targets: Vec<NvDisplayConfigPathTargetInfo> = vec![];
        unsafe {
            for target in std::slice::from_raw_parts(info.targetInfo, info.targetInfoCount as usize)
            {
                targets.push(NvDisplayConfigPathTargetInfo {
                    display_id: target.displayId,
                    details: Box::from_raw(target.details),
                    target_id: target.targetId,
                });
            }
            NvDisplayConfigPathInfo {
                target_info: targets,
                source_mode_info: Box::from_raw(info.sourceModeInfo),
                is_non_nvidia_adapter: info.IsNonNVIDIAAdapter() == 1,
            }
        }
    }
}

pub fn tryCustom() -> Result<()> {
    let mut a = 2147881090;

    let mut timingInput = NV_TIMING_INPUT {
        version: make_nvapi_version::<NV_TIMING_INPUT>(1),
        width: 640,
        height: 480,
        rr: 60.0,
        flag: Default::default(),
        type_: _NV_TIMING_OVERRIDE_NV_TIMING_OVERRIDE_AUTO,
    };

    let mut newTiming: NV_TIMING = Default::default();
    unsafe {
        let result = NvAPI_DISP_GetTiming(2147881090, addr_of_mut!(timingInput), addr_of_mut!(newTiming));
        if result != _NvAPI_Status_NVAPI_OK {
            return Err(format!("{} {}", "Error retrieving timing", get_status_message(&result)))
        }
    }

    let mut customDisplay = NV_CUSTOM_DISPLAY {
        version: make_nvapi_version::<NV_CUSTOM_DISPLAY>(1),
        width: 5120,
        height: 1440,
        depth: 32,
        colorFormat: 21,
        srcPartition: NV_VIEWPORTF {
            x: 0.0,
            y: 0.0,
            w: 1.0,
            h: 1.0,
        },
        xRatio: 1.0,
        yRatio: 1.0,
        timing: newTiming,
        _bitfield_align_1: [],
        _bitfield_1: Default::default(),
        __bindgen_padding_0: [0u8; 3],
    };

    unsafe {
        let result = NvAPI_DISP_TryCustomDisplay(addr_of_mut!(a), 1, addr_of_mut!(customDisplay));
        if result != _NvAPI_Status_NVAPI_OK {
            Err(format!("{} {}", "Error applying resolution", get_status_message(&result)))
        } else {
            Ok(())
        }
    }
}