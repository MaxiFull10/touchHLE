/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIDevice`.

// --- ESTA ES LA PARTE QUE FALTABA ---
// Definimos los módulos hijos para que el compilador sepa que existen.
pub mod ui_accelerometer;
pub mod ui_activity_indicator_view;
pub mod ui_application;
pub mod ui_color;
// pub mod ui_device; // Este NO, porque somos nosotros mismos
pub mod ui_event;
pub mod ui_font;
pub mod ui_geometry;
pub mod ui_graphics;
pub mod ui_image;
pub mod ui_image_picker_controller;
pub mod ui_nib;
pub mod ui_responder;
pub mod ui_screen;
pub mod ui_touch;
pub mod ui_view;
pub mod ui_view_controller;
// ------------------------------------

use crate::dyld::ConstantExports;
use crate::dyld::HostConstant;
use crate::frameworks::foundation::{ns_string, NSInteger};
use crate::objc::{id, msg, objc_classes, ClassExports, TrivialHostObject};
use crate::window::{get_battery_status, BatteryState, DeviceOrientation};

pub const UIDeviceOrientationDidChangeNotification: &str =
    "UIDeviceOrientationDidChangeNotification";

pub type UIDeviceOrientation = NSInteger;
#[allow(dead_code)]
pub const UIDeviceOrientationUnknown: UIDeviceOrientation = 0;
pub const UIDeviceOrientationPortrait: UIDeviceOrientation = 1;
#[allow(dead_code)]
pub const UIDeviceOrientationPortraitUpsideDown: UIDeviceOrientation = 2;
pub const UIDeviceOrientationLandscapeLeft: UIDeviceOrientation = 3;
pub const UIDeviceOrientationLandscapeRight: UIDeviceOrientation = 4;
#[allow(dead_code)]
pub const UIDeviceOrientationFaceUp: UIDeviceOrientation = 5;
#[allow(dead_code)]
pub const UIDeviceOrientationFaceDown: UIDeviceOrientation = 6;

pub type UIDeviceBatteryState = NSInteger;
pub const UIDeviceBatteryStateUnknown: UIDeviceBatteryState = 0;
pub const UIDeviceBatteryStateUnplugged: UIDeviceBatteryState = 1;
pub const UIDeviceBatteryStateCharging: UIDeviceBatteryState = 2;
pub const UIDeviceBatteryStateFull: UIDeviceBatteryState = 3;

#[derive(Default)]
pub struct State {
    current_device: Option<id>,
}

pub const CONSTANTS: ConstantExports = &[(
    "_UIDeviceOrientationDidChangeNotification",
    HostConstant::NSString(UIDeviceOrientationDidChangeNotification),
)];

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIDevice: NSObject

+ (id)currentDevice {
    if let Some(device) = env.framework_state.uikit.ui_device.current_device {
        device
    } else {
        let new = env.objc.alloc_static_object(
            this,
            Box::new(TrivialHostObject),
            &mut env.mem
        );
        env.framework_state.uikit.ui_device.current_device = Some(new);
        new
    }
}

- (())beginGeneratingDeviceOrientationNotifications {}
- (())endGeneratingDeviceOrientationNotifications {}

- (id)model {
    ns_string::get_static_str(env, "iPhone")
}
- (id)localizedModel {
    msg![env; this model]
}

- (id)name {
    ns_string::get_static_str(env, "iPhone")
}

- (id)systemName {
    ns_string::get_static_str(env, "iPhone OS")
}

// NSString
- (id)systemVersion {
    // Parche: 3.0 para compatibilidad
    ns_string::get_static_str(env, "3.0")
}

- (id)uniqueIdentifier {
    // --- PARCHE GEMINI: UDID FIJO ---
    ns_string::get_static_str(env, "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF")
}

- (bool)isMultitaskingSupported {
    false
}

- (UIDeviceOrientation)orientation {
    match env.window().current_rotation() {
        DeviceOrientation::Portrait => UIDeviceOrientationPortrait,
        DeviceOrientation::LandscapeLeft => UIDeviceOrientationLandscapeLeft,
        DeviceOrientation::LandscapeRight => UIDeviceOrientationLandscapeRight
    }
}
- (())setOrientation:(UIDeviceOrientation)orientation {
    env.window_mut().rotate_device(match orientation {
        UIDeviceOrientationPortrait => DeviceOrientation::Portrait,
        UIDeviceOrientationLandscapeLeft => DeviceOrientation::LandscapeLeft,
        UIDeviceOrientationLandscapeRight => DeviceOrientation::LandscapeRight,
        _ => unimplemented!("Orientation {} not handled yet", orientation),
    });
}

- (bool)isBatteryMonitoringEnabled {
    true
}
- (())setBatteryMonitoringEnabled:(bool)enabled {
    assert!(enabled);
}
- (f32)batteryLevel {
    let pct = get_battery_status().0;
    if pct < 0 {
        return 1.0
    }
    pct as f32 / 100.0 
}
- (UIDeviceBatteryState)batteryState {
    match get_battery_status().1 {
        BatteryState::Unknown => UIDeviceBatteryStateUnknown,
        BatteryState::OnBattery => UIDeviceBatteryStateUnplugged,
        BatteryState::NoBattery | BatteryState::Charging => UIDeviceBatteryStateCharging,
        BatteryState::Full => UIDeviceBatteryStateFull,
    }
}

@end

};
