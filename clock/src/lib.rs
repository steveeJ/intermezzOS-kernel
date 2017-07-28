//! The clock crate contains modules to implement the system clock
#![no_std]
#![feature(asm)]
#![feature(const_fn)]
#![deny(unused_variables)]

extern crate spin;
extern crate x86;

pub type Frequency = u32;
pub type SimpleResult<T> = Result<T, &'static str>;
pub struct Duration {
    pub sec: u64,
    pub nsec: u64,
}

/// This module defines constants for the crate level
pub mod consts {
    pub const NSEC_MULTIPLIER: u64 = 1_000_000_000;
    pub const NSEC_DIGITS: usize = 9;
}

impl Duration {
    fn hms(self) -> SimpleResult<(u64,u64,u64)> {
        let h = self.sec / 3600;
        let m = (self.sec % 3600) / 60;
        let s = (self.sec % 3600) % 60;
        Ok((h, m, s))
    }
}

impl core::ops::Sub for Duration {
    type Output = Duration;

    fn sub(self, other: Duration) -> Duration {
        let mut sec = self.sec - other.sec;
        let mut nsec;
        if self.nsec > self.nsec {
            nsec = self.nsec - other.nsec;
        } else if self.nsec == other.nsec {
            nsec = 0;
        } else {
            sec -= 1;
            nsec = other.nsec - self.nsec;
        }

        Duration {
            sec: sec,
            nsec: nsec,
        }
    }
}

impl core::fmt::Display for Duration {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{}.{:0width$}s", self.sec, self.nsec, width=consts::NSEC_DIGITS)
    }
}

/// The Clock trait is for each clock type.
pub trait Clock {
    /// Start the clock
    fn start(&self);

    /// Receive the frequency the clock is set for
    fn frequency(& self) -> SimpleResult<Frequency>;

    /// Update the internal clock counter by one.
    /// The time of one tick is `1/self.frequency()`s.
    fn tick(&self);

    /// Receive the current tick counter
    fn ticks(&self) -> SimpleResult<(u64, Duration)>;

    /// Returns the uptime as `Duration`.
    /// This assumes that **all** fired clock interrupts have successfully called `self.tick()`.
    fn uptime(& self) -> SimpleResult<Duration>;
}

/// This module implements a system clock using the Programmable Interrupt Timer
/// * http://wiki.osdev.org/Programmable_Interval_Timer
/// * https://en.wikibooks.org/wiki/X86_Assembly/Programmable_Interval_Timer
pub mod pit {
    use x86::shared::io::outb;
    use core::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};

    use super::Frequency;
    use super::Duration;
    use super::Clock;
    use super::SimpleResult;
    use super::consts::NSEC_MULTIPLIER;

    /// Constants definitions for the pit module
    pub mod consts {
        pub const BASE_FREQUENCY: u32 = 1193182;
        pub const CHANNEL0_IO_PORT: u16 = 0x40;
        pub const CHANNEL1_IO_PORT: u16 = 0x41;
        pub const CHANNEL2_IO_PORT: u16 = 0x42;
        pub const COMMAND_PORT: u16 = 0x43;
        pub const CHANNEL_IO_PORTS: [u16; 3] = [
            CHANNEL0_IO_PORT,
            CHANNEL1_IO_PORT,
            CHANNEL2_IO_PORT
        ];
    }

    pub struct Pit {
        pub frequency: Frequency,
        divisor: u16,
        pub resolution: u64,
        channel: u8,
        ticks_atomic: AtomicUsize,
    }

    fn gen_command(channel: u8) -> u8 {
        0b11000000 & channel << 6 | // channel
        0b00110000 & 0b11 << 4    | // lobyte/hibyte
        0b00001110 & 0b010 << 1   | // rate generator
        0b00000001 & 0b0            // 16-bit binary mode
    }

    pub fn new(channel: u8, divisor: u16) -> Pit {
        assert!(channel <= 2);

        let freq = consts::BASE_FREQUENCY/(divisor as u32);
        Pit {
            frequency: freq,
            divisor: divisor,
            resolution: NSEC_MULTIPLIER/freq as u64,
            channel: channel,
            ticks_atomic: ATOMIC_USIZE_INIT,
        }
    }

    impl Clock for Pit {
        fn start(&self) {
            let lobyte = (self.divisor & 0xFF) as u8;
            let hibyte = ((self.divisor >> 8) & 0xFF) as u8;
            unsafe {
                outb(consts::COMMAND_PORT, gen_command(self.channel));
                outb(consts::CHANNEL_IO_PORTS[self.channel as usize], lobyte);
                outb(consts::CHANNEL_IO_PORTS[self.channel as usize], hibyte);
            };
        }

        fn frequency(&self) -> SimpleResult<Frequency> {
            Ok(self.frequency)
        }

        fn uptime(&self) -> SimpleResult<Duration> {
            let ticks = self.ticks_atomic.load(Ordering::SeqCst) as u64;
            let sec = ticks / self.frequency as u64;
            let nsec = (ticks - sec * self.frequency as u64) * self.resolution;

            Ok(Duration{
                sec: sec,
                nsec: nsec
            })
        }

        fn ticks(&self) -> SimpleResult<(u64, Duration)> {
            let ticks = self.ticks_atomic.load(Ordering::SeqCst) as u64;
            let sec = ticks / self.frequency as u64;
            let nsec = (ticks - sec * self.frequency as u64) * self.resolution;

            Ok((ticks, Duration{ sec: sec, nsec: nsec }))
        }

        fn tick(&self) {
            self.ticks_atomic.fetch_add(1, Ordering::SeqCst);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Duration;

    #[test]
    fn duration_hms() {
        let d = Duration{sec: 3661, nsec: 5000000};
        assert_eq!(d.hms().unwrap(),(1, 1, 1));
    }

    fn duration_sub() {
        assert_eq!(true, false);
    }
}