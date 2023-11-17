//! This example shows how to communicate asynchronous using i2c with external chips.
//!
//! Example written for the [`MCP23017 16-Bit I2C I/O Expander with Serial Interface`] chip.
//! (https://www.microchip.com/en-us/product/mcp23017)

#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

mod time;
mod wifi;

use crate::wifi::{net_task, wifi_task};
use cyw43_pio::PioSpi;
use defmt::*;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_net::{Config as NetConfig, Stack, StackResources};
use embassy_rp::{
    bind_interrupts,
    gpio::{Level, Output},
    peripherals::PIO0,
    pio::{InterruptHandler as PioInterruptHandler, Pio},
};
use embassy_time::{Duration, Timer};
use panic_probe as _;
use static_cell::make_static;

bind_interrupts!(struct Irqs {
    // I2C1_IRQ => I2cInterruptHandler<I2C1>;
    PIO0_IRQ_0 => PioInterruptHandler<PIO0>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let clock = make_static!(time::Clock::new());

    // --- WIFI
    let wifi_ssid: &'static str = option_env!("WIFI_NETWORK").unwrap_or("test");
    let wifi_pass: &'static str = option_env!("WIFI_PASSWORD").unwrap_or("test");

    let fw = include_bytes!("../firmware/43439A0.bin");
    let clm = include_bytes!("../firmware/43439A0_clm.bin");

    // let fw = unsafe { core::slice::from_raw_parts(0x10100000 as *const u8, 230321) };
    // let clm = unsafe { core::slice::from_raw_parts(0x10140000 as *const u8, 4752) };

    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);
    let mut pio = Pio::new(p.PIO0, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        pio.irq0,
        cs,
        p.PIN_24,
        p.PIN_29,
        p.DMA_CH0,
    );

    let state = make_static!(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;

    spawner.spawn(wifi_task(runner)).unwrap();

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    let config = NetConfig::dhcpv4(Default::default());

    // Generate random seed
    let seed = 0x0123_4567_89ab_cdef; // chosen by fair dice roll. guarenteed to be random.

    // Init network stack
    let stack = &*make_static!(Stack::new(
        net_device,
        config,
        make_static!(StackResources::<4>::new()),
        seed
    ));

    spawner.spawn(net_task(stack)).unwrap();

    loop {
        match control.join_wpa2(wifi_ssid, wifi_pass).await {
            Ok(_) => break,
            Err(err) => {
                info!("join failed with status={}", err.status);
            }
        }
    }

    // spawner.spawn(net_worker(stack)).unwrap();
    // --------

    info!("waiting for DHCP...");
    while !stack.is_config_up() {
        Timer::after(Duration::from_millis(100)).await;
    }
    info!("DHCP is now up!");

    spawner.spawn(time::ntp_worker(stack, clock)).unwrap();
    spawner.spawn(time::time_logger(clock)).unwrap();

    loop {
        Timer::after(Duration::from_secs(60)).await;
    }
}
