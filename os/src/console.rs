// os/src/console.rs
use crate::sbi::console_putchar;
use core::fmt::{self, Write};

struct Stdout;

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            console_putchar(c as usize);
        }
        Ok(())
    }
}

pub fn print(args: fmt::Arguments) {
    Stdout.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!($fmt $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! println {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! println_green {
    ($fmt:literal $(, $($arg:tt)+)?) => {
        $crate::console::print(format_args!(concat!("\x1b[32m", $fmt, "\x1b[0m", "\n") $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! println_yellow {
    ($fmt:literal $(, $($arg:tt)+)?) => {
        $crate::console::print(format_args!(concat!("\x1b[33m", $fmt, "\x1b[0m", "\n") $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! println_red {
    ($fmt:literal $(, $($arg:tt)+)?) => {
        $crate::console::print(format_args!(concat!("\x1b[31m", $fmt, "\x1b[0m", "\n") $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! println_blue {
    ($fmt:literal $(, $($arg:tt)+)?) => {
        $crate::console::print(format_args!(concat!("\x1b[34m", $fmt, "\x1b[0m", "\n") $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! println_gray {
    ($fmt:literal $(, $($arg:tt)+)?) => {
        $crate::console::print(format_args!(concat!("\x1b[37m", $fmt, "\x1b[0m", "\n") $(, $($arg)+)?));
    }
}