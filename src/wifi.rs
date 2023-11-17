use cyw43_pio::PioSpi;
use embassy_net::Stack;
use embassy_rp::{
    gpio::Output,
    peripherals::{DMA_CH0, PIN_23, PIN_25, PIO0},
};

#[embassy_executor::task]
pub async fn wifi_task(
    runner: cyw43::Runner<
        'static,
        Output<'static, PIN_23>,
        PioSpi<'static, PIN_25, PIO0, 0, DMA_CH0>,
    >,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
pub async fn net_task(stack: &'static Stack<cyw43::NetDriver<'static>>) -> ! {
    stack.run().await
}
