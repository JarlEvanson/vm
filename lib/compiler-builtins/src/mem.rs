use core::ffi::c_void;

#[unsafe(no_mangle)]
unsafe extern "C" fn memcpy(
    target: *mut c_void,
    source: *const c_void,
    length: usize,
) -> *mut c_void {
    assert!(length < isize::MAX as usize);

    let target = target.cast::<u8>();
    let source = source.cast::<u8>();

    for index in 0..length {
        // SAFETY:
        //
        // The invariants of this function fulfill the invariants of `target.add()`.
        let target = unsafe { target.add(index) };

        // SAFETY:
        //
        // The invariants of this function fulfill the invariants of `source.add()`.
        let source = unsafe { source.add(index) };

        // SAFETY:
        //
        // The invariants of this function fulfill the invariants of `source.read()`.
        let value = unsafe { source.read() };

        // SAFETY:
        //
        // The invariants of this function fulfill the invariants of `target.write()`.
        unsafe { target.write(value) }
    }

    target.cast::<c_void>()
}

#[unsafe(no_mangle)]
unsafe extern "C" fn memmove(
    target: *mut c_void,
    source: *const c_void,
    length: usize,
) -> *mut c_void {
    assert!(length < isize::MAX as usize);

    let target = target.cast::<u8>();
    let source = source.cast::<u8>();

    if target.addr() > source.addr() && target.addr() < source.addr() + length {
        for index in (0..length).rev() {
            // SAFETY:
            //
            // The invariants of this function fulfill the invariants of `target.add()`.
            let target = unsafe { target.add(index) };

            // SAFETY:
            //
            // The invariants of this function fulfill the invariants of `source.add()`.
            let source = unsafe { source.add(index) };

            // SAFETY:
            //
            // The invariants of this function fulfill the invariants of `source.read()`.
            let value = unsafe { source.read() };

            // SAFETY:
            //
            // The invariants of this function fulfill the invariants of `target.write()`.
            unsafe { target.write(value) }
        }
    } else {
        for index in 0..length {
            // SAFETY:
            //
            // The invariants of this function fulfill the invariants of `target.add()`.
            let target = unsafe { target.add(index) };

            // SAFETY:
            //
            // The invariants of this function fulfill the invariants of `source.add()`.
            let source = unsafe { source.add(index) };

            // SAFETY:
            //
            // The invariants of this function fulfill the invariants of `source.read()`.
            let value = unsafe { source.read() };

            // SAFETY:
            //
            // The invariants of this function fulfill the invariants of `target.write()`.
            unsafe { target.write(value) }
        }
    }

    target.cast::<c_void>()
}

#[unsafe(no_mangle)]
unsafe extern "C" fn memset(
    target: *mut c_void,
    value: core::ffi::c_int,
    length: usize,
) -> *mut c_void {
    assert!(length < isize::MAX as usize);

    let target = target.cast::<u8>();
    let value = value as u8;

    for index in 0..length {
        // SAFETY:
        //
        // The invariants of this function fulfill the invariants of `target.add()`.
        let target = unsafe { target.add(index) };

        // SAFETY:
        //
        // The invariants of this function fulfill the invariants of `target.write()`.
        unsafe { target.write(value) }
    }

    target.cast::<c_void>()
}

#[unsafe(no_mangle)]
unsafe extern "C" fn memcmp(
    source_0: *const c_void,
    source_1: *const c_void,
    length: usize,
) -> core::ffi::c_int {
    assert!(length < isize::MAX as usize);

    let source_0 = source_0.cast::<u8>();
    let source_1 = source_1.cast::<u8>();

    for index in 0..length {
        // SAFETY:
        //
        // The invariants of this function fulfill the invariants of `source_0.add()`.
        let source_0 = unsafe { source_0.add(index) };

        // SAFETY:
        //
        // The invariants of this function fulfill the invariants of `source_1.add()`.
        let source_1 = unsafe { source_1.add(index) };

        // SAFETY:
        //
        // The invariants of this function fulfill the invariants of `source_0.read()`.
        let value_0 = unsafe { source_0.read() as core::ffi::c_int };

        // SAFETY:
        //
        // The invariants of this function fulfill the invariants of `source_1.write()`.
        let value_1 = unsafe { source_1.read() as core::ffi::c_int };

        if value_0 != value_1 {
            return value_0.wrapping_sub(value_1);
        }
    }

    0
}

#[unsafe(no_mangle)]
unsafe extern "C" fn strlen(str: *const core::ffi::c_char) -> usize {
    let mut str_ptr = str.cast::<u8>();
    let mut length: usize = 0;

    // SAFETY:
    //
    // The invariants of this function fulfill the invariants of `str_ptr.read()`.
    while unsafe { str_ptr.read() } != 0 {
        assert!(length < isize::MAX as usize);

        // SAFETY:
        //
        // The invariants of this function fulfill the invariants of `str_ptr.add()`.
        str_ptr = unsafe { str_ptr.add(1) };
        length += 1;
    }

    length
}
