pub mod migemo;

// WASM用のコードブロック
#[cfg(feature = "wasm")]
mod wasm_exports {
    use super::migemo::{compact_dictionary::CompactDictionary, query::query, regex_generator::RegexOperator};
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    #[derive(Debug)]
    pub struct Migemo {
        dictionary: CompactDictionary,
    }

    #[wasm_bindgen]
    impl Migemo {
        #[wasm_bindgen(constructor)]
        pub fn new(buffer: &[u8]) -> Migemo {
            let vec = Vec::from(buffer);
            Migemo {
                dictionary: CompactDictionary::new(&vec),
            }
        }

        pub fn query(&self, word: String) -> String {
            let rxop = RegexOperator::Default;
            query(word, &self.dictionary, &rxop)
        }
    }
}

#[cfg(feature = "wasm")]
pub use wasm_exports::*;

// Windows DLL用のコードブロック
#[cfg(feature = "windows-dll")]
mod windows_exports {
    use super::migemo::compact_dictionary::*;
    // インポートするquery関数に`migemo_query`という別名を付けます
    use super::migemo::query::query as migemo_query;
    use super::migemo::regex_generator::*;

    use std::ffi::CString;
    use std::os::raw::c_char;

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
    pub unsafe extern "C" fn query(migemo: *mut Migemo, buffer: *const u8, len: u32) -> bool {
        if !(*migemo).result_ptr.is_null() {
            let _ = CString::from_raw((*migemo).result_ptr);
            (*migemo).result_ptr = std::ptr::null_mut();
            (*migemo).result_len = 0;
        }

        let mut dst = Vec::with_capacity(len as usize);
        dst.set_len(len as usize);
        std::ptr::copy(buffer, dst.as_mut_ptr(), len as usize);
        let string = String::from_utf8_lossy(&dst);

        let dict = &*(*migemo).dict;
        let rxop = RegexOperator::Default;
        // 別名を付けた`migemo_query`関数を呼び出します
        let result = migemo_query(string.to_string(), dict, &rxop);

        let c_string = CString::new(result).unwrap();
        (*migemo).result_len = c_string.as_bytes().len() as u32;
        (*migemo).result_ptr = c_string.into_raw();

        return true;
    }

    #[no_mangle]
    pub unsafe extern "C" fn destroy(migemo: *mut Migemo) {
        if !(*migemo).result_ptr.is_null() {
            let _ = CString::from_raw((*migemo).result_ptr);
            (*migemo).result_ptr = std::ptr::null_mut();
            (*migemo).result_len = 0;
        }
        if !(*migemo).dict.is_null() {
            let _ = Box::from_raw((*migemo).dict);
            (*migemo).dict = std::ptr::null_mut();
        }
    }
}

#[cfg(feature = "windows-dll")]
pub use windows_exports::*;
