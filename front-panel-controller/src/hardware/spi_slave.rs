use embassy_futures::yield_now;
use embassy_stm32::{
    gpio::{AfType, Flex, OutputType, Pull, Speed},
    pac,
    peripherals::*,
    spi::{CsPin, MisoPin, MosiPin, SckPin},
    Peri,
};

pub struct SpiSlave {
    _peri: Peri<'static, SPI1>,
}

impl SpiSlave {
    pub fn new(
        peri: Peri<'static, SPI1>,
        sck: Peri<'static, PA5>,
        mosi: Peri<'static, PB5>,
        miso: Peri<'static, PA6>,
        nss: Peri<'static, PA15>,
    ) -> Self {
        pac::RCC.apb2enr().modify(|w| w.set_spi1en(true));
        pac::RCC.apb2rstr().modify(|w| w.set_spi1rst(true));
        pac::RCC.apb2rstr().modify(|w| w.set_spi1rst(false));

        let sck_af = <PA5 as SckPin<SPI1>>::af_num(&*sck);
        let mosi_af = <PB5 as MosiPin<SPI1>>::af_num(&*mosi);
        let miso_af = <PA6 as MisoPin<SPI1>>::af_num(&*miso);
        let nss_af = <PA15 as CsPin<SPI1>>::af_num(&*nss);

        let mut sck_flex = Flex::new(sck);
        sck_flex.set_as_af_unchecked(sck_af, AfType::input(Pull::None));
        core::mem::forget(sck_flex);

        let mut mosi_flex = Flex::new(mosi);
        mosi_flex.set_as_af_unchecked(mosi_af, AfType::input(Pull::None));
        core::mem::forget(mosi_flex);

        let mut miso_flex = Flex::new(miso);
        miso_flex.set_as_af_unchecked(miso_af, AfType::output(OutputType::PushPull, Speed::High));
        core::mem::forget(miso_flex);

        let mut nss_flex = Flex::new(nss);
        nss_flex.set_as_af_unchecked(nss_af, AfType::input(Pull::Up));
        core::mem::forget(nss_flex);

        let r = pac::SPI1;

        r.cfg1().modify(|w| {
            w.set_dsize(7);
        });

        r.cfg2().modify(|w| {
            w.set_cpha(pac::spi::vals::Cpha::SECOND_EDGE);
            w.set_cpol(pac::spi::vals::Cpol::IDLE_HIGH);
            w.set_master(pac::spi::vals::Master::SLAVE);
            w.set_ssm(false);
            w.set_ssoe(false);
            w.set_lsbfirst(pac::spi::vals::Lsbfirst::MSBFIRST);
            w.set_comm(pac::spi::vals::Comm::FULL_DUPLEX);
        });

        r.cr1().modify(|w| w.set_spe(true));

        Self { _peri: peri }
    }

    pub async fn transfer(&mut self, rx_buf: &mut [u8], tx_buf: &[u8]) -> Result<(), SpiError> {
        if rx_buf.len() != tx_buf.len() {
            return Err(SpiError::BufferLengthMismatch);
        }

        let len = rx_buf.len();
        if len == 0 {
            return Ok(());
        }

        let r = pac::SPI1;

        while !r.sr().read().txp() {
            yield_now().await;
        }
        r.txdr8().write_value(tx_buf[0]);

        for i in 0..len {
            while !r.sr().read().rxp() {
                yield_now().await;
            }
            let sr = r.sr().read();
            if sr.ovr() {
                let _ = r.rxdr8().read();
                r.ifcr().write(|w| w.set_ovrc(true));
                return Err(SpiError::Overrun);
            }
            rx_buf[i] = r.rxdr8().read();

            if i + 1 < len {
                while !r.sr().read().txp() {
                    yield_now().await;
                }
                r.txdr8().write_value(tx_buf[i + 1]);
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpiError {
    BufferLengthMismatch,
    Overrun,
}
