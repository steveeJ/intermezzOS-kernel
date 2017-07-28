/// Prints something to the screen, with a trailing newline.
///
/// # Examples
///
/// ```ignore
/// kprintln!("Hello, world!");
/// ```
#[macro_export]
macro_rules! kprintln {
    ($ctx:ident, $fmt:expr) => (kprint!($ctx, concat!($fmt, "\n")));
    ($ctx:ident, $fmt:expr, $($arg:tt)*) => (kprint!($ctx, concat!($fmt, "\n"), $($arg)*));
}

/// Prints something to the screen.
///
/// # Examples
///
/// ```ignore
/// kprint!("Hello, world!");
/// ```
#[macro_export]
macro_rules! kprint {
    ($ctx:ident, $($arg:tt)*) => ({
        use core::fmt::Write;
        let mut vga = $ctx.vga.lock();
        vga.write_fmt(format_args!($($arg)*)).unwrap();
        vga.flush();
    });
}

/// Prints something to the screen if it's free to use
///
/// # Examples
///
/// ```ignore
/// kprint_try!("Hello, world!\n");
/// ```
#[macro_export]
macro_rules! kprint_try {
    ($ctx:ident, $($arg:tt)*) => ({
        use core::fmt::Write;
        let vga_guard = $ctx.vga.try_lock();

        if vga_guard.is_none() {
            return
        }

        let mut vga = vga_guard.unwrap();
        vga.write_fmt(format_args!($($arg)*)).unwrap();
        vga.flush();
    });
}

/// Overwrite the current character
///
/// # Examples
///
/// ```ignore
/// kprint!("Hello, world!");
/// ```
#[macro_export]
macro_rules! koverprint {
    ($ctx:ident, $($arg:tt)*) => ({
        use core::fmt::Write;
        let mut vga = $ctx.vga.lock();
        vga.write_fmt(format_args!($($arg)*)).unwrap();
        vga.flush();
    });
}