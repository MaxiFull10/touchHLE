#![allow(warnings)]
/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSFileManager` etc.

use super::{ns_array, ns_string, NSUInteger};
use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant};
use crate::frameworks::foundation::ns_string::get_static_str;
use crate::fs::{GuestPath, GuestPathBuf};
use crate::mem::{ConstPtr, MutPtr, Ptr};
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, ClassExports, HostObject,
};
use crate::Environment;

type NSSearchPathDirectory = NSUInteger;
const NSApplicationDirectory: NSSearchPathDirectory = 1;
const NSLibraryDirectory: NSSearchPathDirectory = 5;
const NSDocumentDirectory: NSSearchPathDirectory = 9;

type NSSearchPathDomainMask = NSUInteger;
const NSUserDomainMask: NSSearchPathDomainMask = 1;

pub const NSFileModificationDate: &str = "NSFileModificationDate";
pub const NSFileSize: &str = "NSFileSize";
const NSFileSystemFreeSize: &str = "NSFileSystemFreeSize";

pub const CONSTANTS: ConstantExports = &[
    (
        "_NSFileModificationDate",
        HostConstant::NSString(NSFileModificationDate),
    ),
    ("_NSFileSize", HostConstant::NSString(NSFileSize)),
    (
        "_NSFileSystemFreeSize",
        HostConstant::NSString(NSFileSystemFreeSize),
    ),
];

fn NSSearchPathForDirectoriesInDomains(
    env: &mut Environment,
    directory: NSSearchPathDirectory,
    domain_mask: NSSearchPathDomainMask,
    expand_tilde: bool,
) -> id {
    assert!(domain_mask == NSUserDomainMask);
    assert!(expand_tilde);

    let dir = match directory {
        NSApplicationDirectory => {
            GuestPath::new(crate::fs::APPLICATIONS).to_owned()
        }
        NSDocumentDirectory => env.fs.home_directory().join("Documents"),
        NSLibraryDirectory => env.fs.home_directory().join("Library"),
        _ => todo!("NSSearchPathDirectory {}", directory),
    };
    
    let dir = ns_string::from_rust_string(env, String::from(dir));
    let dir_list = ns_array::from_vec(env, vec![dir]);
    autorelease(env, dir_list)
}

fn NSHomeDirectory(env: &mut Environment) -> id {
    let dir = env.fs.home_directory();
    let dir = ns_string::from_rust_string(env, String::from(dir.as_str()));
    autorelease(env, dir)
}

fn NSTemporaryDirectory(env: &mut Environment) -> id {
    let dir = env.fs.home_directory().join("tmp");
    let dir = ns_string::from_rust_string(env, String::from(dir.as_str()));
    autorelease(env, dir)
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(NSHomeDirectory()),
    export_c_func!(NSTemporaryDirectory()),
    export_c_func!(NSSearchPathForDirectoriesInDomains(_, _, _)),
];

#[derive(Default)]
pub struct State {
    default_manager: Option<id>,
}

struct NSDirectoryEnumeratorHostObject {
    iterator: std::vec::IntoIter<GuestPathBuf>,
}
impl HostObject for NSDirectoryEnumeratorHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSFileManager: NSObject

+ (id)defaultManager {
    if let Some(existing) = env.framework_state.foundation.ns_file_manager.default_manager {
        existing
    } else {
        let new: id = msg![env; this new];
        env.framework_state.foundation.ns_file_manager.default_manager = Some(new);
        new
    }
}

- (id)currentDirectoryPath {
    ns_string::from_rust_string(env, env.fs.working_directory().as_str().to_string())
}

- (bool)changeCurrentDirectoryPath:(id)path {
    let path = ns_string::to_rust_string(env, path); 
    let path = GuestPath::new(&path);
    match env.fs.change_working_directory(path) {
        Ok(_) => true,
        Err(()) => false
    }
}

- (bool)fileExistsAtPath:(id)path { 
    let res_exists = if path == nil {
        false
    } else {
        let path_str = ns_string::to_rust_string(env, path); 
        let exists = env.fs.exists(GuestPath::new(&path_str));
        exists
    };
    res_exists
}

- (bool)fileExistsAtPath:(id)path 
             isDirectory:(MutPtr<bool>)is_dir {
    let (res_exists, res_is_dir) = if path == nil {
        (false, false)
    } else {
        let path_str = ns_string::to_rust_string(env, path); 
        let guest_path = GuestPath::new(&path_str);
        let exists = env.fs.exists(guest_path);
        (exists, !env.fs.is_file(guest_path))
    };

    if !is_dir.is_null() {
        env.mem.write(is_dir, res_is_dir);
    }
    res_exists
}

// --- ESCRITURA LIMPIA (DELETE + WRITE RAW) ---
- (bool)createFileAtPath:(id)path 
                contents:(id)data 
              attributes:(id)attributes { 
    assert!(attributes == nil); 

    let path_str = ns_string::to_rust_string(env, path); 
    let guest_path = GuestPath::new(&path_str);

    // 1. Preparar los datos
    let empty_data: &[u8] = &[];
    let data_slice = if data == nil {
        empty_data
    } else {
        let bytes_ptr: ConstPtr<u8> = msg![env; data bytes];
        let length: NSUInteger = msg![env; data length];
        env.mem.bytes_at(bytes_ptr, length as usize)
    };

    // 2. BORRADO PREVENTIVO (Crucial para evitar corrupción al final del archivo)
    if env.fs.exists(guest_path) {
        // Intentamos borrar. Si falla (ej: archivo en uso), seguimos adelante
        // con la esperanza de que el write lo trunque, pero el remove es más seguro.
        let _ = env.fs.remove(guest_path);
    }

    // 3. ESCRITURA FRESCA
    // Al haber borrado (o al usar write que trunca), el archivo se crea de 0
    // con el tamaño exacto de los nuevos datos. Sin basura al final.
    match env.fs.write(guest_path, data_slice) {
        Ok(_) => true,
        Err(_) => false,
    }
}
// --------------------------------------------

- (bool)removeItemAtPath:(id)path 
                   error:(MutPtr<id>)error { 
    let path = ns_string::to_rust_string(env, path); 
    match env.fs.remove(GuestPath::new(&path)) {
        Ok(()) => true,
        Err(()) => {
            if !error.is_null() {
                todo!(); 
            }
            false
        }
    }
}

- (bool)createDirectoryAtPath:(id)path 
                   attributes:(id)attributes { 
    let error: MutPtr<id> = Ptr::null();
    msg![env; this createDirectoryAtPath:path
             withIntermediateDirectories:false
                              attributes:attributes
                                   error:error]
}

- (bool)createDirectoryAtPath:(id)path 
  withIntermediateDirectories:(bool)with_intermediates
                   attributes:(id)attributes 
                        error:(MutPtr<id>)error { 
    assert_eq!(attributes, nil); 

    let path_str = ns_string::to_rust_string(env, path); 
    let res = if with_intermediates {
        env.fs.create_dir_all(GuestPath::new(&path_str))
    } else {
        env.fs.create_dir(GuestPath::new(&path_str))
    };
    match res {
        Ok(()) => {
            true
        }
        Err(err) => {
            assert!(error.is_null()); 
            // log error
            false
        }
    }
}

- (id)enumeratorAtPath:(id)path { 
    let path = ns_string::to_rust_string(env, path); 
    let Ok(paths) = env.fs.enumerate_recursive(GuestPath::new(&path)) else {
        return nil;
    };
    let host_object = Box::new(NSDirectoryEnumeratorHostObject {
        iterator: paths.into_iter(),
    });
    let class = env.objc.get_known_class("NSDirectoryEnumerator", &mut env.mem);
    let enumerator = env.objc.alloc_object(class, host_object, &mut env.mem);
    autorelease(env, enumerator)
}

- (id)directoryContentsAtPath:(id)path { 
    let path_str = ns_string::to_rust_string(env, path); 
    let Ok(paths) = env.fs.enumerate(GuestPath::new(&path_str)) else {
        return nil;
    };
    let paths: Vec<GuestPathBuf> = paths
        .map(|path| GuestPathBuf::from(GuestPath::new(path)))
        .collect();
    
    let path_strings = paths
        .iter()
        .map(|name| ns_string::from_rust_string(env, name.as_str().to_string()))
        .collect();
    let res = ns_array::from_vec(env, path_strings);
    autorelease(env, res)
}

- (id)contentsOfDirectoryAtPath:(id)path 
                          error:(MutPtr<id>)error { 
    let contents: id = msg![env; this directoryContentsAtPath:path];
    if contents == nil && !error.is_null() {
        todo!(); 
    }
    contents
}

- (bool)isReadableFileAtPath:(id)_path { 
    true
}

- (bool)isWritableFileAtPath:(id)_path { 
    true
}

- (bool)isDeletableFileAtPath:(id)_path { 
    true
}

- (id)contentsAtPath:(id)path { 
    assert!(msg![env; path isAbsolutePath]);
    msg_class![env; NSData dataWithContentsOfFile:path]
}

- (bool)copyItemAtPath:(id)src 
                toPath:(id)dst 
                 error:(MutPtr<id>)error { 
    let src = ns_string::to_rust_string(env, src);
    let dst = ns_string::to_rust_string(env, dst);
    let data = match env.fs.read(GuestPath::new(src.as_ref())) {
        Ok(d) => d,
        Err(_) => {
            assert!(error.is_null()); 
            return false;
        }
    };
    if env.fs.write(GuestPath::new(dst.as_ref()), &data).is_err() {
        assert!(error.is_null()); 
        return false;
    }
    true
}

- (ConstPtr<u8>)fileSystemRepresentationWithPath:(id)path { 
    let length: NSUInteger = msg![env; path length];
    assert!(length > 0);
    msg![env; path UTF8String]
}

- (id)fileAttributesAtPath:(id)path 
              traverseLink:(bool)_traverse {
    let path_str = ns_string::to_rust_string(env, path); 
    let guest_path = GuestPath::new(&path_str);
    file_attributes_common(env, guest_path)
}

- (id)attributesOfItemAtPath:(id)path 
                       error:(MutPtr<id>)error { 
    assert!(error.is_null()); 
    let path_str = ns_string::to_rust_string(env, path); 
    let guest_path = GuestPath::new(&path_str);
    file_attributes_common(env, guest_path)
}

- (id)attributesOfFileSystemForPath:(id)_path
                              error:(MutPtr<id>)error {
    assert!(error.is_null()); 

    let dict = msg_class![env; NSMutableDictionary new];

    let size: u64 = 1024 * 1024 * 1024;
    let size_num: id = msg_class![env; NSNumber numberWithUnsignedLongLong:size];

    let fs_free_size_key = get_static_str(env, NSFileSystemFreeSize);
    () = msg![env; dict setObject:size_num forKey:fs_free_size_key];

    let dict_imm = msg![env; dict copy];
    release(env, dict);
    autorelease(env, dict_imm)
}

@end

@implementation NSDirectoryEnumerator: NSEnumerator

- (id)nextObject {
    let host_obj = env.objc.borrow_mut::<NSDirectoryEnumeratorHostObject>(this);
    host_obj.iterator.next().map_or(nil, |s| ns_string::from_rust_string(env, String::from(s)))
}

@end

};

fn file_attributes_common(env: &mut Environment, guest_path: &GuestPath) -> id {
    if !env.fs.exists(guest_path) {
        return nil;
    }

    let is_file = env.fs.is_file(guest_path);
    
    let unix_timestamp: f64 = env.fs.modified(guest_path).unwrap() as f64;
    let unix_ref_date: id = msg_class![env; NSDate dateWithTimeIntervalSince1970:0f64];
    let unix_date: id =
        msg_class![env; NSDate dateWithTimeInterval:unix_timestamp sinceDate:unix_ref_date];

    let size = env.fs.size(guest_path).unwrap();
    let size_num: id = msg_class![env; NSNumber numberWithUnsignedLongLong:size];

    let dict = msg_class![env; NSMutableDictionary new];

    let modif_date_key = get_static_str(env, NSFileModificationDate);
    () = msg![env; dict setObject:unix_date forKey:modif_date_key];

    let size_key = get_static_str(env, NSFileSize);
    () = msg![env; dict setObject:size_num forKey:size_key];

    let type_key = ns_string::from_rust_string(env, String::from("NSFileType"));
    let type_val_str = if is_file { "NSFileTypeRegular" } else { "NSFileTypeDirectory" };
    let type_val = ns_string::from_rust_string(env, String::from(type_val_str));
    () = msg![env; dict setObject:type_val forKey:type_key];

    let perm_key = ns_string::from_rust_string(env, String::from("NSFilePosixPermissions"));
    let perm_val: id = msg_class![env; NSNumber numberWithInt:511];
    () = msg![env; dict setObject:perm_val forKey:perm_key];

    let dict_imm = msg![env; dict copy];
    release(env, dict);
    autorelease(env, dict_imm)
}    if data == nil {
        let empty: id = msg_class![env; NSData new];
        let res: bool = msg![env; empty writeToFile:path atomically:false];
        release(env, empty);
        res
    } else {
        msg![env; data writeToFile:path atomically:false]
    }
}
// -----------------------------------------------------------

- (bool)removeItemAtPath:(id)path 
                   error:(MutPtr<id>)error { 
    let path = ns_string::to_rust_string(env, path); 
    match env.fs.remove(GuestPath::new(&path)) {
        Ok(()) => true,
        Err(()) => {
            if !error.is_null() {
                todo!(); 
            }
            false
        }
    }
}

- (bool)createDirectoryAtPath:(id)path 
                   attributes:(id)attributes { 
    let error: MutPtr<id> = Ptr::null();
    msg![env; this createDirectoryAtPath:path
             withIntermediateDirectories:false
                              attributes:attributes
                                   error:error]
}

- (bool)createDirectoryAtPath:(id)path 
  withIntermediateDirectories:(bool)with_intermediates
                   attributes:(id)attributes 
                        error:(MutPtr<id>)error { 
    assert_eq!(attributes, nil); 

    let path_str = ns_string::to_rust_string(env, path); 
    let res = if with_intermediates {
        env.fs.create_dir_all(GuestPath::new(&path_str))
    } else {
        env.fs.create_dir(GuestPath::new(&path_str))
    };
    match res {
        Ok(()) => true,
        Err(_) => {
            assert!(error.is_null()); 
            false
        }
    }
}

- (id)enumeratorAtPath:(id)path { 
    let path = ns_string::to_rust_string(env, path); 
    let Ok(paths) = env.fs.enumerate_recursive(GuestPath::new(&path)) else {
        return nil;
    };
    let host_object = Box::new(NSDirectoryEnumeratorHostObject {
        iterator: paths.into_iter(),
    });
    let class = env.objc.get_known_class("NSDirectoryEnumerator", &mut env.mem);
    let enumerator = env.objc.alloc_object(class, host_object, &mut env.mem);
    autorelease(env, enumerator)
}

- (id)directoryContentsAtPath:(id)path { 
    let path_str = ns_string::to_rust_string(env, path); 
    let Ok(paths) = env.fs.enumerate(GuestPath::new(&path_str)) else {
        return nil;
    };
    let paths: Vec<GuestPathBuf> = paths
        .map(|path| GuestPathBuf::from(GuestPath::new(path)))
        .collect();
    
    let path_strings = paths
        .iter()
        .map(|name| ns_string::from_rust_string(env, name.as_str().to_string()))
        .collect();
    let res = ns_array::from_vec(env, path_strings);
    autorelease(env, res)
}

- (id)contentsOfDirectoryAtPath:(id)path 
                          error:(MutPtr<id>)error { 
    let contents: id = msg![env; this directoryContentsAtPath:path];
    if contents == nil && !error.is_null() {
        todo!(); 
    }
    contents
}

// Añadido guion bajo (_) para silenciar warnings
- (bool)isReadableFileAtPath:(id)_path { 
    true
}

- (bool)isWritableFileAtPath:(id)_path { 
    true
}

- (bool)isDeletableFileAtPath:(id)_path { 
    true
}

- (id)contentsAtPath:(id)path { 
    assert!(msg![env; path isAbsolutePath]);
    msg_class![env; NSData dataWithContentsOfFile:path]
}

- (bool)copyItemAtPath:(id)src 
                toPath:(id)dst 
                 error:(MutPtr<id>)error { 
    let src = ns_string::to_rust_string(env, src);
    let dst = ns_string::to_rust_string(env, dst);
    let data = match env.fs.read(GuestPath::new(src.as_ref())) {
        Ok(d) => d,
        Err(_) => {
            assert!(error.is_null()); 
            return false;
        }
    };
    if env.fs.write(GuestPath::new(dst.as_ref()), &data).is_err() {
        assert!(error.is_null()); 
        return false;
    }
    true
}

- (ConstPtr<u8>)fileSystemRepresentationWithPath:(id)path { 
    let length: NSUInteger = msg![env; path length];
    assert!(length > 0);
    msg![env; path UTF8String]
}

// Añadido guion bajo (_) para silenciar warnings
- (id)fileAttributesAtPath:(id)path 
              traverseLink:(bool)_traverse {
    let path_str = ns_string::to_rust_string(env, path); 
    let guest_path = GuestPath::new(&path_str);
    file_attributes_common(env, guest_path)
}

- (id)attributesOfItemAtPath:(id)path 
                       error:(MutPtr<id>)error { 
    assert!(error.is_null()); 
    let path_str = ns_string::to_rust_string(env, path); 
    let guest_path = GuestPath::new(&path_str);
    file_attributes_common(env, guest_path)
}

- (id)attributesOfFileSystemForPath:(id)_path
                              error:(MutPtr<id>)error {
    assert!(error.is_null()); 

    let dict = msg_class![env; NSMutableDictionary new];

    let size: u64 = 1024 * 1024 * 1024;
    let size_num: id = msg_class![env; NSNumber numberWithUnsignedLongLong:size];

    let fs_free_size_key = get_static_str(env, NSFileSystemFreeSize);
    () = msg![env; dict setObject:size_num forKey:fs_free_size_key];

    let dict_imm = msg![env; dict copy];
    release(env, dict);
    autorelease(env, dict_imm)
}

@end

@implementation NSDirectoryEnumerator: NSEnumerator

- (id)nextObject {
    let host_obj = env.objc.borrow_mut::<NSDirectoryEnumeratorHostObject>(this);
    host_obj.iterator.next().map_or(nil, |s| ns_string::from_rust_string(env, String::from(s)))
}

@end

};

/// Helper function V4
fn file_attributes_common(env: &mut Environment, guest_path: &GuestPath) -> id {
    if !env.fs.exists(guest_path) {
        return nil;
    }

    let is_file = env.fs.is_file(guest_path);
    
    let unix_timestamp: f64 = env.fs.modified(guest_path).unwrap() as f64;
    let unix_ref_date: id = msg_class![env; NSDate dateWithTimeIntervalSince1970:0f64];
    let unix_date: id =
        msg_class![env; NSDate dateWithTimeInterval:unix_timestamp sinceDate:unix_ref_date];

    let size = env.fs.size(guest_path).unwrap();
    let size_num: id = msg_class![env; NSNumber numberWithUnsignedLongLong:size];

    let dict = msg_class![env; NSMutableDictionary new];

    let modif_date_key = get_static_str(env, NSFileModificationDate);
    () = msg![env; dict setObject:unix_date forKey:modif_date_key];

    let size_key = get_static_str(env, NSFileSize);
    () = msg![env; dict setObject:size_num forKey:size_key];

    // PARCHE DE ATRIBUTOS
    let type_key = ns_string::from_rust_string(env, String::from("NSFileType"));
    let type_val_str = if is_file { "NSFileTypeRegular" } else { "NSFileTypeDirectory" };
    let type_val = ns_string::from_rust_string(env, String::from(type_val_str));
    () = msg![env; dict setObject:type_val forKey:type_key];

    let perm_key = ns_string::from_rust_string(env, String::from("NSFilePosixPermissions"));
    let perm_val: id = msg_class![env; NSNumber numberWithInt:511];
    () = msg![env; dict setObject:perm_val forKey:perm_key];

    let dict_imm = msg![env; dict copy];
    release(env, dict);
    autorelease(env, dict_imm)
}
