#![no_std]
#![no_main]

use aya_bpf::{macros::tracepoint, programs::TracePointContext, BpfContext};
use aya_log_ebpf::{debug, error, info, trace, warn};

#[tracepoint]
pub fn example(ctx: TracePointContext) -> u32 {
    error!(&ctx, "this is an error message 🚨");
    warn!(&ctx, "this is a warning message ⚠️");
    info!(&ctx, "this is an info message ℹ️");
    debug!(&ctx, "this is a debug message ️🐝");
    trace!(&ctx, "this is a trace message 🔍");
    let pid = ctx.pid();
    info!(&ctx, "a message with args PID: {}", pid);
    let ip = 1575522155u32;
    info!(&ctx, "IP address (as int): {}", ip);
    info!(&ctx, "IP address (human readable): {:ipv4}", ip);
    info!(&ctx, "number (base 10): {}", 42);
    info!(&ctx, "number (hex): {:x}", 42);
    info!(&ctx, "number (upper hex): {:X}", 42);
    0
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
