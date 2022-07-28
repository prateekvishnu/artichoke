use std::ffi::CStr;
use std::os::raw::c_char;
use std::ptr;
use std::slice;

use crate::extn::core::symbol::Symbol;
use crate::extn::prelude::*;

// ```c
// MRB_API mrb_sym mrb_intern(mrb_state*,const char*,size_t);
//
#[no_mangle]
unsafe extern "C" fn mrb_intern(mrb: *mut sys::mrb_state, name: *const c_char, len: usize) -> sys::mrb_sym {
    let bytes = slice::from_raw_parts(name.cast::<u8>(), len);
    let bytes = bytes.to_vec();
    unwrap_interpreter!(mrb, to => guard, or_else = 0);
    let sym = guard.intern_bytes(bytes);
    let sym = sym.map(u32::from);
    sym.unwrap_or_default()
}

// ```c
// MRB_API mrb_sym mrb_intern_static(mrb_state*,const char*,size_t);
// ```
#[no_mangle]
unsafe extern "C" fn mrb_intern_static(mrb: *mut sys::mrb_state, name: *const c_char, len: usize) -> sys::mrb_sym {
    let bytes = slice::from_raw_parts::<'static, _>(name.cast::<u8>(), len);
    unwrap_interpreter!(mrb, to => guard, or_else = 0);
    let sym = guard.intern_bytes(bytes);
    let sym = sym.map(u32::from);
    sym.unwrap_or_default()
}

// ```c
// MRB_API mrb_sym mrb_intern_cstr(mrb_state *mrb, const char* str);
// ```
#[no_mangle]
unsafe extern "C" fn mrb_intern_cstr(mrb: *mut sys::mrb_state, name: *const c_char) -> sys::mrb_sym {
    let string = CStr::from_ptr(name);
    let bytes = string.to_bytes_with_nul().to_vec();
    unwrap_interpreter!(mrb, to => guard, or_else = 0);
    let sym = guard.intern_bytes_with_trailing_nul(bytes);
    let sym = sym.map(u32::from);
    sym.unwrap_or_default()
}

// ```c
// MRB_API mrb_sym mrb_intern_str(mrb_state*,mrb_value);
// ```
#[no_mangle]
unsafe extern "C" fn mrb_intern_str(mrb: *mut sys::mrb_state, name: sys::mrb_value) -> sys::mrb_sym {
    unwrap_interpreter!(mrb, to => guard, or_else = 0);
    let name = Value::from(name);
    if let Ok(bytes) = name.try_convert_into_mut::<Vec<u8>>(&mut guard) {
        let sym = guard.intern_bytes(bytes);
        let sym = sym.map(u32::from);
        sym.unwrap_or_default()
    } else {
        0
    }
}

/* `mrb_intern_check` series functions returns 0 if the symbol is not defined */

// ```c
// MRB_API mrb_sym mrb_intern_check(mrb_state*,const char*,size_t);
// ```
#[no_mangle]
unsafe extern "C" fn mrb_intern_check(mrb: *mut sys::mrb_state, name: *const c_char, len: usize) -> sys::mrb_sym {
    let bytes = slice::from_raw_parts(name.cast::<u8>(), len);
    unwrap_interpreter!(mrb, to => guard, or_else = 0);
    if let Ok(Some(sym)) = guard.check_interned_bytes(bytes) {
        sym
    } else {
        0
    }
}

// ```c
// MRB_API mrb_sym mrb_intern_check_cstr(mrb_state*,const char*);
// ```
#[no_mangle]
unsafe extern "C" fn mrb_intern_check_cstr(mrb: *mut sys::mrb_state, name: *const c_char) -> sys::mrb_sym {
    let string = CStr::from_ptr(name);
    let bytes = string.to_bytes();
    unwrap_interpreter!(mrb, to => guard, or_else = 0);
    if let Ok(Some(sym)) = guard.check_interned_bytes(bytes) {
        sym
    } else {
        0
    }
}

// ```c
// MRB_API mrb_sym mrb_intern_check_str(mrb_state*,mrb_value);
// ```
#[no_mangle]
unsafe extern "C" fn mrb_intern_check_str(mrb: *mut sys::mrb_state, name: sys::mrb_value) -> sys::mrb_sym {
    unwrap_interpreter!(mrb, to => guard, or_else = 0);
    let name = Value::from(name);
    if let Ok(bytes) = name.try_convert_into_mut::<&[u8]>(&mut guard) {
        if let Ok(Some(sym)) = guard.check_interned_bytes(bytes) {
            return sym;
        }
    }
    0
}

// `mrb_check_intern` series functions returns `nil` if the symbol is not
// defined; otherwise returns `mrb_value`.

// ```c
// MRB_API mrb_value mrb_check_intern(mrb_state*,const char*,size_t);
// ```
#[no_mangle]
unsafe extern "C" fn mrb_check_intern(mrb: *mut sys::mrb_state, name: *const c_char, len: usize) -> sys::mrb_value {
    let bytes = slice::from_raw_parts(name.cast::<u8>(), len);
    unwrap_interpreter!(mrb, to => guard);
    let symbol = if let Ok(Some(sym)) = guard.check_interned_bytes(bytes) {
        Symbol::alloc_value(sym.into(), &mut guard).unwrap_or_default()
    } else {
        Value::nil()
    };
    symbol.inner()
}

// ```c
// MRB_API mrb_value mrb_check_intern_cstr(mrb_state*,const char*);
// ```
#[no_mangle]
unsafe extern "C" fn mrb_check_intern_cstr(mrb: *mut sys::mrb_state, name: *const c_char) -> sys::mrb_value {
    let string = CStr::from_ptr(name);
    let bytes = string.to_bytes_with_nul();
    unwrap_interpreter!(mrb, to => guard);
    let symbol = if let Ok(Some(sym)) = guard.check_interned_bytes_with_trailing_nul(bytes) {
        Symbol::alloc_value(sym.into(), &mut guard).unwrap_or_default()
    } else {
        Value::nil()
    };
    symbol.inner()
}

// ```c
// MRB_API mrb_value mrb_check_intern_str(mrb_state*,mrb_value);
// ```
#[no_mangle]
unsafe extern "C" fn mrb_check_intern_str(mrb: *mut sys::mrb_state, name: sys::mrb_value) -> sys::mrb_value {
    unwrap_interpreter!(mrb, to => guard);
    let name = Value::from(name);
    let symbol = if let Ok(bytes) = name.try_convert_into_mut::<&[u8]>(&mut guard) {
        if let Ok(Some(sym)) = guard.check_interned_bytes(bytes) {
            Symbol::alloc_value(sym.into(), &mut guard).unwrap_or_default()
        } else {
            Value::nil()
        }
    } else {
        Value::nil()
    };
    symbol.inner()
}

// ```c
// MRB_API const char *mrb_sym_name(mrb_state*,mrb_sym);
// ```
#[no_mangle]
unsafe extern "C" fn mrb_sym_name(mrb: *mut sys::mrb_state, sym: sys::mrb_sym) -> *const c_char {
    unwrap_interpreter!(mrb, to => guard, or_else = ptr::null());
    if let Ok(Some(bytes)) = guard.lookup_symbol_with_trailing_nul(sym) {
        bytes.as_ptr().cast::<c_char>()
    } else {
        ptr::null()
    }
}

// ```c
// MRB_API const char *mrb_sym_name_len(mrb_state*,mrb_sym,mrb_int*);
// ```
#[no_mangle]
unsafe extern "C" fn mrb_sym_name_len(
    mrb: *mut sys::mrb_state,
    sym: sys::mrb_sym,
    lenp: *mut sys::mrb_int,
) -> *const c_char {
    if !lenp.is_null() {
        ptr::write(lenp, 0);
    }
    unwrap_interpreter!(mrb, to => guard, or_else = ptr::null());
    if let Ok(Some(bytes)) = guard.lookup_symbol(sym) {
        if !lenp.is_null() {
            if let Ok(len) = sys::mrb_int::try_from(bytes.len()) {
                ptr::write(lenp, len);
            } else {
                return ptr::null();
            }
        }
        bytes.as_ptr().cast()
    } else {
        ptr::null()
    }
}

// ```c
// MRB_API const char *mrb_sym_dump(mrb_state*,mrb_sym);
// ```
#[no_mangle]
unsafe extern "C" fn mrb_sym_dump(mrb: *mut sys::mrb_state, sym: sys::mrb_sym) -> *const c_char {
    unwrap_interpreter!(mrb, to => guard, or_else = ptr::null());
    if let Ok(Some(bytes)) = guard.lookup_symbol(sym) {
        let mut bytes = bytes.to_vec();

        // SAFETY: ensure the byte buffer is NUL-terminated.
        //
        // Add a NUL byte to the end of the allocation for the byte buffer in
        // the new `RString*`.
        //
        // mruby assumes that symbols are stored in memory as a null terminated
        // `char*`s and creates "static" `RString`s from interned bytes that
        // point at this NUL terminated memory.
        //
        // Sometimes mruby grabs the `RString` pointer directly from value
        // returned by this API call and assumes it is NUL terminated.
        //
        // Add a 0 byte to the end of the `Vec` to ensure that the byte at index
        // `len + 1` is NUL. Then set the length to the length of the `Vec` to
        // hide the NUL byte.
        //
        // See https://github.com/artichoke/artichoke/pull/1969.
        let len = bytes.len();
        bytes.reserve_exact(1);
        bytes.push(0);
        bytes.set_len(len);

        // Allocate a buffer with the lifetime of the interpreter and return
        // a pointer to it.
        if let Ok(string) = guard.try_convert_mut(bytes) {
            if let Ok(bytes) = string.try_convert_into_mut::<&[u8]>(&mut guard) {
                return bytes.as_ptr().cast();
            }
        }
    }
    ptr::null()
}

// ```c
// MRB_API mrb_value mrb_sym_str(mrb_state*,mrb_sym);
// ```
#[no_mangle]
unsafe extern "C" fn mrb_sym_str(mrb: *mut sys::mrb_state, sym: sys::mrb_sym) -> sys::mrb_value {
    unwrap_interpreter!(mrb, to => guard);

    let mut bytes = if let Ok(Some(bytes)) = guard.lookup_symbol(sym) {
        bytes.to_vec()
    } else {
        vec![]
    };

    // SAFETY: ensure the byte buffer is NUL-terminated.
    //
    // Add a NUL byte to the end of the allocation for the byte buffer in
    // the new `RString*`.
    //
    // mruby assumes that symbols are stored in memory as a null terminated
    // `char*`s and creates "static" `RString`s from interned bytes that
    // point at this NUL terminated memory.
    //
    // Sometimes mruby grabs the `RString` pointer directly from value
    // returned by this API call and assumes it is NUL terminated.
    //
    // Add a 0 byte to the end of the `Vec` to ensure that the byte at index
    // `len + 1` is NUL. Then set the length to the length of the `Vec` to
    // hide the NUL byte.
    //
    // See https://github.com/artichoke/artichoke/pull/1969.
    let len = bytes.len();
    bytes.reserve_exact(1);
    bytes.push(0);
    bytes.set_len(len);

    let value = guard.try_convert_mut(bytes);
    value.unwrap_or_default().inner()
}

// ```c
// void mrb_init_symtbl(mrb_state*);
// ```
#[no_mangle]
unsafe extern "C" fn mrb_init_symtbl(mrb: *mut sys::mrb_state) {
    // The symbol table is initialized before the call to `mrb_open_allocf` in
    // `crate::interpreter::interpreter`. This function is intended to be called
    // during the initialization of the `mrb_state`.
    let _ = mrb;
}

// ```c
// void mrb_free_symtbl(mrb_state *mrb);
// ```
#[no_mangle]
unsafe extern "C" fn mrb_free_symtbl(mrb: *mut sys::mrb_state) {
    // The symbol table is freed when the Rust `State` is freed.
    let _ = mrb;
}
