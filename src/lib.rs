mod migemo;
use migemo::compact_dictionary::*;
use migemo::query::*;
use migemo::regex_generator::*;

use std::mem;
use std::os::raw::c_char;
use std::ffi::CString;

#[repr(C, align(8))]
pub struct Migemo {
    dict: *mut CompactDictionary,
    result_ptr: *mut c_char,
    result_len: u32,
}

#[no_mangle]
pub unsafe extern "C" fn load(buffer: *const u8, len: u32) -> Migemo {
    let mut dst = Vec::with_capacity(len as usize);
    dst.set_len(len as usize);
    std::ptr::copy(buffer, dst.as_mut_ptr(), len as usize);
    let dict = Box::new(CompactDictionary::new(&dst));
    return Migemo {
        dict: Box::into_raw(dict),
        result_ptr: 0 as *mut c_char,
        result_len: 0,
    };
}

#[no_mangle]
pub unsafe extern "C" fn query(mut migemo: *mut Migemo, buffer: *const u8, len: u32) -> bool {
    if (*migemo).result_ptr as usize != 0 {
        unsafe { Box::from_raw((*migemo).result_ptr) };
        (*migemo).result_ptr = 0 as *mut c_char;
        (*migemo).result_len = 0;
    }
    let mut dst = Vec::with_capacity(len as usize);
    dst.set_len(len as usize);
    std::ptr::copy(buffer, dst.as_mut_ptr(), len as usize);
    let string = String::from_utf8_lossy(&dst);
    let dict = unsafe { &*(*migemo).dict };
    let rxop = RegexOperator::Default;
    let result = migemo::query::query(string.to_string(), &dict, &rxop);
    (*migemo).result_len = result.len() as u32;
    (*migemo).result_ptr = CString::new(result).unwrap().into_raw();
    return true;
}

#[no_mangle]
pub unsafe extern "C" fn destroy(mut migemo: *mut Migemo) {
    if (*migemo).result_ptr as usize != 0 {
        unsafe { Box::from_raw((*migemo).result_ptr) };
        (*migemo).result_ptr = 0 as *mut c_char;
        (*migemo).result_len = 0;
    }
    unsafe { Box::from_raw((*migemo).dict) };
    (*migemo).dict = 0 as *mut CompactDictionary;
}