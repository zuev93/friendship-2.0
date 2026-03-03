use embassy_stm32::peripherals as stm_peripherals;
use embassy_stm32::usb::{self, Driver};
use embassy_stm32::{interrupt, Peri};
use embassy_usb::class::cdc_acm::{CdcAcmClass, State as CdcState};
use embassy_usb::class::uac1::speaker::{Speaker, State as SpeakerState};
use embassy_usb::class::uac1::{Channel, FeedbackRefresh, SampleWidth};
use embassy_usb::{Builder, Config, UsbDevice};
use static_cell::StaticCell;

type UsbDriver = Driver<'static, stm_peripherals::USB>;

pub struct Usb {
    pub device: UsbDevice<'static, UsbDriver>,
    pub cat_cdc: CdcAcmClass<'static, UsbDriver>,
    pub cli_cdc: CdcAcmClass<'static, UsbDriver>,
    pub stream: embassy_usb::class::uac1::speaker::Stream<'static, UsbDriver>,
    pub feedback: embassy_usb::class::uac1::speaker::Feedback<'static, UsbDriver>,
    pub control_monitor: embassy_usb::class::uac1::speaker::ControlMonitor<'static>,
}

impl Usb {
    pub fn new(
        usb: Peri<'static, stm_peripherals::USB>,
        dp: Peri<'static, stm_peripherals::PA12>,
        dm: Peri<'static, stm_peripherals::PA11>,
        irqs: impl interrupt::typelevel::Binding<
                <stm_peripherals::USB as usb::Instance>::Interrupt,
                usb::InterruptHandler<stm_peripherals::USB>,
            > + 'static,
    ) -> Self {
        let driver = Driver::new(usb, irqs, dp, dm);

        let mut config = Config::new(0x1209, 0x0001);
        config.manufacturer = Some("Druzhba");
        config.product = Some("Druzhba-M Transceiver");
        config.device_class = 0xEF;
        config.device_sub_class = 0x02;
        config.device_protocol = 0x01;
        config.composite_with_iads = true;
        config.max_power = 100;
        config.max_packet_size_0 = 64;

        static CONFIG_DESC: StaticCell<[u8; 256]> = StaticCell::new();
        static BOS_DESC: StaticCell<[u8; 256]> = StaticCell::new();
        static MSOS_DESC: StaticCell<[u8; 0]> = StaticCell::new();
        static CONTROL_BUF: StaticCell<[u8; 128]> = StaticCell::new();

        let config_desc = CONFIG_DESC.init([0; 256]);
        let bos_desc = BOS_DESC.init([0; 256]);
        let msos_desc = MSOS_DESC.init([]);
        let control_buf = CONTROL_BUF.init([0; 128]);

        let mut builder = Builder::new(
            driver,
            config,
            config_desc,
            bos_desc,
            msos_desc,
            control_buf,
        );

        static CAT_STATE: StaticCell<CdcState> = StaticCell::new();
        let cat_state = CAT_STATE.init(CdcState::new());
        let cat_cdc = CdcAcmClass::new(&mut builder, cat_state, 64);

        static CLI_STATE: StaticCell<CdcState> = StaticCell::new();
        let cli_state = CLI_STATE.init(CdcState::new());
        let cli_cdc = CdcAcmClass::new(&mut builder, cli_state, 64);

        static SPEAKER_STATE: StaticCell<SpeakerState> = StaticCell::new();
        let speaker_state = SPEAKER_STATE.init(SpeakerState::new());
        static CHANNELS: [Channel; 1] = [Channel::LeftFront];
        let (stream, feedback, control_monitor) = Speaker::new(
            &mut builder,
            speaker_state,
            192,
            SampleWidth::Width2Byte,
            &[48000],
            &CHANNELS,
            FeedbackRefresh::Period8Frames,
        );

        let device = builder.build();

        Self {
            device,
            cat_cdc,
            cli_cdc,
            stream,
            feedback,
            control_monitor,
        }
    }
}
