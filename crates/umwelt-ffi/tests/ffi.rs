// SPDX-License-Identifier: AGPL-3.0-only

use std::path::Path;
use std::ptr;

use serde_json::{Value, json};
use umwelt_ffi::*;

fn open(path: &Path, create: bool) -> (i32, *mut umwelt_engine) {
    let path = path.to_str().expect("temporary path should be UTF-8");
    let mut handle = ptr::null_mut();
    let status = unsafe { umwelt_open(path.as_ptr(), path.len(), i32::from(create), &mut handle) };
    (status, handle)
}

fn call(handle: *mut umwelt_engine, request: Value) -> (i32, Value) {
    let request = request.to_string();
    let mut output = umwelt_buf {
        ptr: ptr::null_mut(),
        len: 0,
    };
    let status = unsafe { umwelt_call(handle, request.as_ptr(), request.len(), &mut output) };
    if output.ptr.is_null() {
        return (status, Value::Null);
    }
    let bytes = unsafe { std::slice::from_raw_parts(output.ptr, output.len) };
    let response = serde_json::from_slice(bytes).expect("response should be JSON");
    unsafe {
        umwelt_buf_free(output);
    }
    (status, response)
}

fn last_error(handle: *mut umwelt_engine) -> Value {
    let mut output = umwelt_buf {
        ptr: ptr::null_mut(),
        len: 0,
    };
    assert_eq!(unsafe { umwelt_last_error(handle, &mut output) }, 0);
    let bytes = unsafe { std::slice::from_raw_parts(output.ptr, output.len) };
    let error = serde_json::from_slice(bytes).expect("last error should be JSON");
    unsafe {
        umwelt_buf_free(output);
    }
    error
}

fn close(handle: *mut umwelt_engine) {
    unsafe {
        umwelt_close(handle);
    }
}

fn free(buffer: umwelt_buf) {
    unsafe {
        umwelt_buf_free(buffer);
    }
}

#[test]
fn ffi_runs_protocol_lifecycle_and_frees_buffers() {
    assert_eq!(umwelt_abi_version(), 1);
    let directory = tempfile::tempdir().expect("temporary directory should exist");
    let (status, handle) = open(&directory.path().join("ffi.umw"), true);
    assert_eq!(status, 0);
    assert!(!handle.is_null());
    let (_, hello) = call(
        handle,
        json!({"v":1,"id":"h","op":"hello","payload":{"protocol":{"min":1,"max":1},"client":"ffi-test/1"}}),
    );
    assert_eq!(hello["ok"], true);
    let (_, project) = call(
        handle,
        json!({"v":1,"id":"p","op":"project.create","payload":{"id":"p","name":"FFI","timescale":"turn","vocabularyMode":"open"}}),
    );
    assert_eq!(project["result"]["version"], 0);
    call(
        handle,
        json!({"v":1,"id":"t","op":"entityType.create","payload":{"projectId":"p","id":"person","name":"Person","kind":"agent"}}),
    );
    call(
        handle,
        json!({"v":1,"id":"e1","op":"entity.create","payload":{"projectId":"p","id":"alice","typeId":"person","name":"Alice"}}),
    );
    call(
        handle,
        json!({"v":1,"id":"e2","op":"entity.create","payload":{"projectId":"p","id":"bob","typeId":"person","name":"Bob"}}),
    );
    call(
        handle,
        json!({"v":1,"id":"d","op":"predicate.define","payload":{"projectId":"p","predicate":{"id":"trusts","label":"Trusts","object":"entity","cardinality":"many"}}}),
    );
    let provenance = json!({"origin":{"kind":"system","name":"ffi-test"}});
    let claim = json!({"subject":"bob","predicate":"trusts","object":{"kind":"entity","id":"alice"},"qualifiers":{},"polarity":true});
    let (_, committed) = call(
        handle,
        json!({"v":1,"id":"c","op":"commit","payload":{"projectId":"p","attemptId":"ffi-commit","event":{"id":"event","type":"test","title":"Trust","position":{"tick":1},"provenance":provenance},"records":[{"kind":"holding","id":"holding","position":{"tick":1},"provenance":provenance,"holder":"alice","attitude":"believes","claim":claim,"acquisition":{"kind":"authored"},"confidence":1.0,"accessibility":"available"}],"transitions":[]}}),
    );
    assert_eq!(committed["ok"], true);
    let (_, holds) = call(
        handle,
        json!({"v":1,"id":"q","op":"holds","payload":{"projectId":"p","holder":"alice","claim":claim}}),
    );
    assert_eq!(holds["ok"], true, "{holds}");
    assert_eq!(holds["result"]["holds"], true);
    free(umwelt_buf {
        ptr: ptr::null_mut(),
        len: 0,
    });
    close(handle);
    close(ptr::null_mut());
}

#[test]
fn invalid_arguments_and_utf8_are_reported() {
    let mut output = umwelt_buf {
        ptr: ptr::null_mut(),
        len: 0,
    };
    assert_eq!(
        unsafe { umwelt_call(ptr::null_mut(), ptr::null(), 0, &mut output) },
        1
    );
    let directory = tempfile::tempdir().expect("temporary directory should exist");
    let (_, handle) = open(&directory.path().join("invalid.umw"), true);
    let invalid = [0xff_u8];
    assert_eq!(
        unsafe { umwelt_call(handle, invalid.as_ptr(), invalid.len(), &mut output) },
        1
    );
    assert_eq!(last_error(handle)["code"], "INVALID_ARGUMENT");
    close(handle);
}

#[test]
fn open_failures_return_queryable_failed_handles() {
    let directory = tempfile::tempdir().expect("temporary directory should exist");
    let missing = directory.path().join("missing.umw");
    let (status, failed) = open(&missing, false);
    assert_eq!(status, 2);
    assert_eq!(last_error(failed)["code"], "NOT_FOUND");
    close(failed);

    let locked_path = directory.path().join("locked.umw");
    let (_, first) = open(&locked_path, true);
    let (status, second) = open(&locked_path, false);
    assert_eq!(status, 2);
    assert_eq!(last_error(second)["code"], "FILE_LOCKED");
    close(second);
    close(first);
}

#[cfg(feature = "test-panic")]
#[test]
fn panic_poisons_the_handle() {
    let directory = tempfile::tempdir().expect("temporary directory should exist");
    let (_, handle) = open(&directory.path().join("panic.umw"), true);
    let request = br#"{"v":1,"id":"panic","op":"ffi.testPanic","payload":{}}"#;
    let mut output = umwelt_buf {
        ptr: ptr::null_mut(),
        len: 0,
    };
    assert_eq!(
        unsafe { umwelt_call(handle, request.as_ptr(), request.len(), &mut output) },
        4
    );
    let hello = br#"{"v":1,"id":"hello","op":"hello","payload":{"protocol":{"min":1,"max":1},"client":"test"}}"#;
    assert_eq!(
        unsafe { umwelt_call(handle, hello.as_ptr(), hello.len(), &mut output) },
        3
    );
    assert_eq!(last_error(handle)["code"], "POISONED");
    close(handle);
}

#[test]
fn open_with_options_accepts_both_durabilities_and_rejects_others() {
    let directory = tempfile::tempdir().expect("temporary directory");
    for (name, durability) in [
        ("normal.umw", UMWELT_DURABILITY_NORMAL),
        ("full.umw", UMWELT_DURABILITY_FULL),
    ] {
        let path = directory.path().join(name);
        let path = path.to_str().expect("UTF-8 path");
        let mut handle = ptr::null_mut();
        let status = unsafe {
            umwelt_open_with_options(path.as_ptr(), path.len(), 1, durability, &mut handle)
        };
        assert_eq!(status, 0);
        let (_, hello) = call(
            handle,
            json!({"v":1,"id":"h","op":"hello","payload":{"protocol":{"min":1,"max":1},"client":"ffi-test/1"}}),
        );
        assert_eq!(hello["ok"], true);
        close(handle);
    }
    let path = directory.path().join("invalid.umw");
    let path = path.to_str().expect("UTF-8 path");
    let mut handle = ptr::null_mut();
    let status = unsafe { umwelt_open_with_options(path.as_ptr(), path.len(), 1, 7, &mut handle) };
    assert_eq!(status, 1);
    assert_eq!(last_error(handle)["code"], "INVALID_ARGUMENT");
    close(handle);
    assert!(!directory.path().join("invalid.umw").exists());
}
