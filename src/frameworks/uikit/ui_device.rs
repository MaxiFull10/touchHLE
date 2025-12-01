#![allow(warnings)]
/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIDevice`.

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

- (id)systemVersion {
    ns_string::get_static_str(env, "3.0")
}

- (id)uniqueIdentifier {
    // PARCHE GEMINI: UDID FIJO
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
