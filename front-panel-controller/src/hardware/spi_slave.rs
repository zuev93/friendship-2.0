use embassy_futures::yield_now;
use embassy_stm32::{
    gpio::{
        low_level::{AFType, Pin as _},
        Pull,
    },
    pac,
    peripherals::*,
    spi::{CsPin, MisoPin, MosiPin, SckPin},
};

pub struct SpiSlave {
    _peri: SPI1,
}

impl SpiSlave {
    pub fn new(peri: SPI1, sck: PA5, mosi: PB5, miso: PA6, nss: PA15) -> Self {
        pac::RCC.apb2enr().modify(|w| w.set_spi1en(true));
        pac::RCC.apb2rstr().modify(|w| w.set_spi1rst(true));
        pac::RCC.apb2rstr().modify(|w| w.set_spi1rst(false));

        sck.set_as_af_pull(
            <PA5 as SckPin<SPI1>>::af_num(&sck),
            AFType::Input,
            Pull::None,
        );
        mosi.set_as_af_pull(
            <PB5 as MosiPin<SPI1>>::af_num(&mosi),
            AFType::Input,
            Pull::None,
        );
        miso.set_as_af_pull(
            <PA6 as MisoPin<SPI1>>::af_num(&miso),
            AFType::OutputPushPull,
            Pull::None,
        );
        nss.set_as_af_pull(<PA15 as CsPin<SPI1>>::af_num(&nss), AFType::Input, Pull::Up);

        let r = pac::SPI1;

        r.cr1().modify(|w| {
            w.set_cpha(pac::spi::vals::Cpha::SECONDEDGE);
            w.set_cpol(pac::spi::vals::Cpol::IDLEHIGH);
            w.set_mstr(pac::spi::vals::Mstr::SLAVE);
            w.set_ssi(false);
            w.set_ssm(false);
            w.set_lsbfirst(pac::spi::vals::Lsbfirst::MSBFIRST);
            w.set_br(pac::spi::vals::Br::DIV2);
            w.set_rxonly(pac::spi::vals::Rxonly::FULLDUPLEX);
            w.set_bidimode(pac::spi::vals::Bidimode::UNIDIRECTIONAL);
            w.set_crcen(false);
        });

        r.cr2().modify(|w| {
            w.set_ssoe(false);
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

        while !r.sr().read().txe() {
            yield_now().await;
        }
        r.dr().write(|w| w.set_dr(tx_buf[0] as u16));

        for i in 0..len {
            while !r.sr().read().rxne() {
                yield_now().await;
            }
            let sr = r.sr().read();
            if sr.ovr() {
                let _ = r.dr().read();
                let _ = r.sr().read();
                return Err(SpiError::Overrun);
            }
            rx_buf[i] = r.dr().read().dr() as u8;

            if i + 1 < len {
                while !r.sr().read().txe() {
                    yield_now().await;
                }
                r.dr().write(|w| w.set_dr(tx_buf[i + 1] as u16));
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
