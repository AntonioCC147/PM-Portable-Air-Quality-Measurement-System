// +---------------------------------------------------------------------------+
// |                             PM/MA lab skel                                |
// +---------------------------------------------------------------------------+

//! By default, this app prints a "Hello world" message with `defmt`.

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};

use {defmt_rtt as _, panic_probe as _};
// Use the logging macros provided by defmt.
use defmt::*;

// Import interrupts definition module
mod irqs;

use embassy_rp::{bind_interrupts, i2c::{Config as I2cConfig, I2c, InterruptHandler as I2CInterruptHandler}};
use embedded_hal_async::i2c::{Error, I2c as _};
use embassy_rp::peripherals::I2C1;

use eeprom24x::{Eeprom24x, SlaveAddr};

bind_interrupts!(struct Irqs {
    I2C1_IRQ => I2CInterruptHandler<I2C1>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // Get a handle to the RP's peripherals.
    let peripherals = embassy_rp::init(Default::default());

    info!("Hello world!");

    let sda = peripherals.PIN_6;
    let scl = peripherals.PIN_7;

    let mut i2c = I2c::new_async(peripherals.I2C1, scl, sda, Irqs, I2cConfig::default());

    let delay = Duration::from_millis(1000);

    const target_addr: u8 = 0x76; // Replace with the actual I2C address of your device
    const CTRL_MEAS: u8 = 0xF4; 
    let tx_buf = [CTRL_MEAS, 0b010_000_11];
    i2c.write(target_addr, &tx_buf).await.unwrap();

    info!("I2C write done!");

    // Read calibration data
    const CALIB: u8 = 0x88;
    let mut data = [0u8; 6];
    i2c.write_read(target_addr, &[CALIB], &mut data).await.unwrap();

    info!("I2C read done!");

    let dig_t1: u16 = ((data[1] as u16) << 8) | (data[0] as u16);
    let dig_t2: i16 = ((data[3] as i16) << 8) | (data[2] as i16);
    let dig_t3: i16 = ((data[5] as i16) << 8) | (data[4] as i16);

    let EEPROM_ADDR: u8 = 0x50; // EEPROM I2C address

    let mem_addr: u16 = 0xACDC; // 16 bit address
    let mem_buff: [u8; 2] = mem_addr.to_be_bytes(); // `be` stands for big endian
    let mut data: [u8; 8] = [0, 0, 0, 0,0,0,0,0];

    i2c.write_read(EEPROM_ADDR, &mem_buff, &mut data).await.unwrap();

    info!("EEPROM read done!");
    
    // make temp from EEPROM data
    let mut temp: i64 = ((data[0] as i64) << 56) 
                      + ((data[1] as i64) << 48) 
                      + ((data[2] as i64) << 40) 
                      + ((data[3] as i64) << 32)
                      + ((data[4] as i64) << 24)
                      + ((data[5] as i64) << 16)
                      + ((data[6] as i64) << 8)
                      + (data[7] as i64);

    info!("EEPROM temperature: {}.{}°C", temp / 100, temp.abs() % 100);


    loop {
        Timer::after(delay).await;

        /*let mut rx_buf = [0x00u8; 2];
        match i2c.read(target_addr, & mut rx_buf).await {
            Ok(_) => {
                info!("Read from I2C device: {:#X}", rx_buf[0]);
            }
            Err(e) => {
                error!("I2C read error: {:?}", e);
            }
        }

        // Increment the target address for the next read
        target_addr += 1;
        if target_addr > 0x78 {
            break; // Reset to the starting address
        }*/

        let first_byte_tx_buf = [0xFA];
        let mut first_byte_rx_buf = [0x00u8];
        i2c.write_read(target_addr, &first_byte_tx_buf, &mut first_byte_rx_buf).await.unwrap();

        let second_byte_tx_buf = [0xFB];
        let mut second_byte_rx_buf = [0x00u8];
        i2c.write_read(target_addr, &second_byte_tx_buf, &mut second_byte_rx_buf).await.unwrap();

        let third_byte_tx_buf = [0xFC];
        let mut third_byte_rx_buf = [0x00u8];
        i2c.write_read(target_addr, &third_byte_tx_buf, &mut third_byte_rx_buf).await.unwrap();

        /*info!("Read from I2C device: {:#X}", first_byte_rx_buf[0]);
        info!("Read from I2C device: {:#X}", second_byte_rx_buf[0]);
        info!("Read from I2C device: {:#X}", third_byte_rx_buf[0]);*/

        let raw_temp: i32 = ((first_byte_rx_buf[0] as i32) << 12) 
                          + ((second_byte_rx_buf[0] as i32) << 4) 
                          + ((third_byte_rx_buf[0] as i32) >> 4);

            

        //info!("Raw temperature: {:#X}", raw_temp);

        let var1 = (((raw_temp >> 3) - ((dig_t1 as i32) << 1)) * (dig_t2 as i32)) >> 11;
        let var2 = (((((raw_temp >> 4) - (dig_t1 as i32)) * ((raw_temp >> 4) - (dig_t1 as i32))) >> 12) * (dig_t3 as i32)) >> 14;
        let t_fine = var1 + var2;
    
        let actual_temp = (t_fine * 5 + 128) >> 8;

        info!(
            "Temperature {}.{}°C",
            actual_temp / 100,
            actual_temp.abs() % 100
        );

        let mem_addr: u16 = 0xACDC; // 16 bit address
        let mem_buff: [u8; 2] = mem_addr.to_be_bytes(); // `be` stands for big endian


        // write actual temperature to EEPROM
        let mut temp_buf= [0x00; 8 + 2];
        temp_buf[..2].copy_from_slice(&mem_buff);
        temp_buf[2..].copy_from_slice(&(actual_temp as i64).to_be_bytes());
        i2c.write(EEPROM_ADDR, &temp_buf).await.unwrap();
        

    }
}
