#![no_std]
#![no_main]

use aya_bpf::{macros::tracepoint, programs::TracePointContext, BpfContext};
use aya_log_ebpf::{debug, error, info, trace, warn};

#[tracepoint]
pub fn example(ctx: TracePointContext) -> u32 {
    error!(&ctx, "this is an error message đ¨");
    warn!(&ctx, "this is a warning message â ī¸");
    info!(&ctx, "this is an info message âšī¸");
    debug!(&ctx, "this is a debug message ī¸đ");
    trace!(&ctx, "this is a trace message đ");
    let pid = ctx.pid();
    info!(&ctx, "a message with args PID: {}", pid);
    0
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
