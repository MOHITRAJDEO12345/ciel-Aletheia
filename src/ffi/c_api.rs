use crate::engine::session::Session;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};

#[no_mangle]
pub extern "C" fn aletheia_session_create(initial_fuel: u64) -> *mut Session {
    Box::into_raw(Box::new(Session::new(initial_fuel)))
}

#[no_mangle]
pub extern "C" fn aletheia_session_free(session: *mut Session) {
    if !session.is_null() {
        unsafe { drop(Box::from_raw(session)) };
    }
}

#[no_mangle]
pub extern "C" fn aletheia_session_fork(session: *mut Session) -> *mut Session {
    if session.is_null() {
        return std::ptr::null_mut();
    }
    let s = unsafe { &*session };
    Box::into_raw(Box::new(s.fork()))
}

#[no_mangle]
pub extern "C" fn aletheia_verify_math(
    gold: *const c_char,
    pred: *const c_char,
) -> c_int {
    if gold.is_null() || pred.is_null() {
        return -1;
    }
    let gold_str = match unsafe { CStr::from_ptr(gold) }.to_str() {
        Ok(s) => s,
        Err(_) => return -2,
    };
    let pred_str = match unsafe { CStr::from_ptr(pred) }.to_str() {
        Ok(s) => s,
        Err(_) => return -2,
    };
    let res = crate::engine::math_verify::verify_math(gold_str, pred_str);
    if res.verified {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn aletheia_eval(
    session: *mut Session,
    code: *const c_char,
    out_buf: *mut c_char,
    out_len: c_int,
) -> c_int {
    if session.is_null() || code.is_null() || out_buf.is_null() || out_len <= 0 {
        return -1;
    }

    let c_str = unsafe { CStr::from_ptr(code) };
    let code_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return -2,
    };

    let s = unsafe { &mut *session };
    match s.eval(code_str) {
        Ok(val) => {
            let res_str = format!("{:?}", val);
            let c_res = CString::new(res_str).unwrap_or_default();
            let bytes = c_res.as_bytes_with_nul();
            if bytes.len() > out_len as usize {
                return -3; // Buffer too small
            }
            unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr() as *const c_char, out_buf, bytes.len());
            }
            0 // Success
        }
        Err(e) => {
            let err_str = e.to_string();
            let c_err = CString::new(err_str).unwrap_or_default();
            let bytes = c_err.as_bytes_with_nul();
            let max_len = (out_len as usize).saturating_sub(1);
            let copy_len = bytes.len().min(max_len);
            unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr() as *const c_char, out_buf, copy_len);
                *out_buf.add(copy_len) = 0;
            }
            1 // Trapped / Error
        }
    }
}
