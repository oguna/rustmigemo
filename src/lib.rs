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
        /// Throws a JavaScript `Error` if the dictionary data is malformed.
        #[wasm_bindgen(constructor)]
        pub fn new(buffer: &[u8]) -> Result<Migemo, JsError> {
            let dictionary = CompactDictionary::try_new(buffer).map_err(|e| JsError::new(&e.to_string()))?;
            Ok(Migemo { dictionary })
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
    use std::panic::{AssertUnwindSafe, catch_unwind};

    #[repr(C, align(8))]
    pub struct Migemo {
        dict: *mut CompactDictionary,
        result_ptr: *mut c_char,
        result_len: u32,
    }

    /// Builds a byte slice from a C pointer/length pair, returning `None` if
    /// `buffer` is NULL when `len > 0`, or if `len` exceeds `isize::MAX`.
    unsafe fn bytes<'a>(buffer: *const u8, len: u32) -> Option<&'a [u8]> {
        let len = usize::try_from(len).ok()?;
        if len == 0 {
            Some(&[])
        } else if buffer.is_null() || len > isize::MAX as usize {
            None
        } else {
            Some(unsafe { std::slice::from_raw_parts(buffer, len) })
        }
    }

    /// Frees the previous result and stores `result` (always a valid C string).
    unsafe fn set_result(migemo: &mut Migemo, result: CString) {
        if !migemo.result_ptr.is_null() {
            drop(unsafe { CString::from_raw(migemo.result_ptr) });
        }
        migemo.result_len = result.as_bytes().len() as u32;
        migemo.result_ptr = result.into_raw();
    }

    /// Loads a dictionary. On failure (NULL buffer or malformed data) the returned
    /// handle has a NULL `dict`, and `query` on it returns `false`.
    ///
    /// # Safety
    ///
    /// When `len > 0`, `buffer` must be valid for reads of `len` bytes for the
    /// duration of this call (or NULL, in which case loading fails safely).
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn load(buffer: *const u8, len: u32) -> Migemo {
        let dict = unsafe { bytes(buffer, len) }
            .and_then(|data| catch_unwind(|| CompactDictionary::try_new(data).ok()).ok().flatten())
            .map_or(std::ptr::null_mut(), |d| Box::into_raw(Box::new(d)));
        Migemo {
            dict,
            result_ptr: std::ptr::null_mut(),
            result_len: 0,
        }
    }

    /// Runs a query. Returns `false` on error (NULL arguments, no dictionary, or a
    /// query containing NUL); `result_ptr` is then an empty string, never dangling.
    ///
    /// # Safety
    ///
    /// `migemo` must be NULL or a valid pointer to a [`Migemo`] returned by [`load`]
    /// that has not been destroyed. When `len > 0`, `buffer` must be NULL or valid
    /// for reads of `len` bytes.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn query(migemo: *mut Migemo, buffer: *const u8, len: u32) -> bool {
        let Some(migemo) = (unsafe { migemo.as_mut() }) else {
            return false;
        };
        let result = (|| {
            let dict = unsafe { migemo.dict.as_ref() }?;
            let input = unsafe { bytes(buffer, len) }?;
            let string = String::from_utf8_lossy(input).into_owned();
            // Never unwind across the FFI boundary: that aborts the host process.
            let pattern =
                catch_unwind(AssertUnwindSafe(|| migemo_query(string, dict, &RegexOperator::Default))).ok()?;
            CString::new(pattern).ok()
        })();
        let ok = result.is_some();
        unsafe { set_result(migemo, result.unwrap_or_default()) };
        ok
    }

    /// Frees the dictionary and last query result owned by `migemo`.
    ///
    /// # Safety
    ///
    /// `migemo` must be NULL or a valid pointer to a [`Migemo`] returned by [`load`].
    /// Calling `destroy` more than once on the same handle is safe.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn destroy(migemo: *mut Migemo) {
        let Some(migemo) = (unsafe { migemo.as_mut() }) else {
            return;
        };
        if !migemo.result_ptr.is_null() {
            drop(unsafe { CString::from_raw(migemo.result_ptr) });
            migemo.result_ptr = std::ptr::null_mut();
            migemo.result_len = 0;
        }
        if !migemo.dict.is_null() {
            drop(unsafe { Box::from_raw(migemo.dict) });
            migemo.dict = std::ptr::null_mut();
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::ffi::CStr;

        fn result(m: &Migemo) -> String {
            unsafe { CStr::from_ptr(m.result_ptr) }.to_str().unwrap().to_string()
        }

        #[test]
        fn query_and_destroy() {
            let data = std::fs::read("todofuken").unwrap();
            let mut m = unsafe { load(data.as_ptr(), data.len() as u32) };
            assert!(!m.dict.is_null());
            let q = b"oosaka";
            assert!(unsafe { query(&mut m, q.as_ptr(), q.len() as u32) });
            assert!(result(&m).contains("大阪府"));
            assert_eq!(m.result_len as usize, result(&m).len());
            unsafe { destroy(&mut m) };
            unsafe { destroy(&mut m) }; // idempotent
        }

        #[test]
        fn invalid_inputs_do_not_crash() {
            let mut bad = unsafe { load([0x7f, 0xff, 0xff, 0xff].as_ptr(), 4) };
            assert!(bad.dict.is_null());
            assert!(!unsafe { query(&mut bad, b"a".as_ptr(), 1) });
            assert_eq!(result(&bad), "");
            unsafe { destroy(&mut bad) };

            let null = unsafe { load(std::ptr::null(), 10) };
            assert!(null.dict.is_null());
            assert!(!unsafe { query(std::ptr::null_mut(), b"a".as_ptr(), 1) });
            unsafe { destroy(std::ptr::null_mut()) };

            let data = std::fs::read("todofuken").unwrap();
            let mut m = unsafe { load(data.as_ptr(), data.len() as u32) };
            assert!(!unsafe { query(&mut m, b"a\0b".as_ptr(), 3) });
            assert_eq!(result(&m), "");
            assert!(!unsafe { query(&mut m, std::ptr::null(), 5) });
            unsafe { destroy(&mut m) };
        }
    }
}

#[cfg(feature = "windows-dll")]
pub use windows_exports::*;
