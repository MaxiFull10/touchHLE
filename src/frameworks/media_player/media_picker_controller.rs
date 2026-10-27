/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `MPMediaPickerController`.
//!
//! There is no media library in touchHLE, so this is only a stub that lets
//! apps create and configure a picker without crashing.

use crate::frameworks::foundation::NSUInteger;
use crate::objc::{id, msg, nil, objc_classes, todo_objc_setter, ClassExports};

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation MPMediaPickerController: UIViewController

- (id)initWithMediaTypes:(NSUInteger)media_types {
    log!(
        "TODO: [(MPMediaPickerController*){:?} initWithMediaTypes:{}] (stub)",
        this,
        media_types
    );
    msg![env; this initWithNibName:nil bundle:nil]
}

- (())setDelegate:(id)delegate {
    todo_objc_setter!(this, delegate);
}

- (())setAllowsPickingMultipleItems:(bool)allows {
    todo_objc_setter!(this, allows);
}

- (())setPrompt:(id)prompt { // NSString *
    todo_objc_setter!(this, prompt);
}

@end

};
