use core::cell::RefCell;
use core::sync::atomic::{AtomicU32, Ordering};
use cortex_m::interrupt::Mutex as CsMutex;

const CPU_MHZ: u32 = 250;

#[repr(usize)]
#[derive(Clone, Copy)]
pub enum TaskId {
    Alc,
    Buttons,
    Encoder,
    SweepScheduler,
    ToneControl,
    CwKeyer,
    Scan,
    IdleCounter,
    Audio,
    Controls,
    DspControls,
    VoxControls,

    ModeArbiter,
    ToneArbiter,
    TransmitModeArbiter,
    FilterArbiter,
    FrequencyArbiter,
    IfGainModeArbiter,
    RfGainModeArbiter,
    ClarifierModeArbiter,
    NbArbiter,
    NrArbiter,
    AnfArbiter,
    DspFilterArbiter,
    CwPeakArbiter,
    AudioAgcArbiter,
    DspBandwidthArbiter,
    DspShiftArbiter,
    CwPeakWidthArbiter,
    CwPitchArbiter,
    TxEqArbiter,
    RxEqArbiter,
    CompressionArbiter,
    VoxArbiter,
    ScanArbiter,
    CwKeyerArbiter,
    VolumeArbiter,
    MicrophoneArbiter,
    RfPowerArbiter,
    IfGainArbiter,
    ClarifierValueArbiter,
    SquelchArbiter,
    NbLevelArbiter,
    NrLevelArbiter,

    MixerTasks,
    CrystalFilter,
    NbActivity,
    IfGainControl,
    RssiRead,
    DetectorTasks,
    AudioPanelSaiRx,
    AudioPanelSaiTx,
    AudioPanelControl,

    BpfTask,
    LpfControl,
    LpfData,
    HfAmpControl,
    HfAmpThermal,

    PowerControl,
    PowerMonitor,
    AudioI2sSpeakers,
    ControlBoardAudioControl,
    UcpdTask,
    StatusLed,
    FanControl,
    Ptt,

    UsbDevice,
    CatTask,
    CliTask,
    SpeakerTask,

    SpiReceiver,
    RadioState,
    WaterfallSender,
    MenuSender,
    FrontPanelAudioMic,
    FrontPanelAudioHeadphones,
    FrontPanelAudioControl,
    CursorTask,

    RitModeLed,
    RfGainModeLed,
    TransmitModeLed,
    TransmitLed,
    ToneLed,
    ModeLed,
    AgcModeLed,

    StatsTask,
    Watchdog,

    COUNT,
}

#[derive(Clone, Copy)]
pub struct TaskStats {
    pub last_us: u32,
    pub max_us: u32,
    pub avg_us: u32,
    pub call_count: u32,
}

impl TaskStats {
    pub const fn new() -> Self {
        Self {
            last_us: 0,
            max_us: 0,
            avg_us: 0,
            call_count: 0,
        }
    }

    fn record(&mut self, cycles: u32) {
        let us = cycles / CPU_MHZ;
        self.last_us = us;
        if us > self.max_us {
            self.max_us = us;
        }
        self.avg_us = self.avg_us - self.avg_us / 8 + us / 8;
        self.call_count = self.call_count.wrapping_add(1);
    }
}

pub static TASK_STATS: CsMutex<RefCell<[TaskStats; TaskId::COUNT as usize]>> =
    CsMutex::new(RefCell::new([TaskStats::new(); TaskId::COUNT as usize]));

pub fn record(task_id: TaskId, cycles: u32) {
    cortex_m::interrupt::free(|cs| {
        TASK_STATS.borrow(cs).borrow_mut()[task_id as usize].record(cycles);
    });
}

pub static IDLE_CAL: AtomicU32 = AtomicU32::new(1);

pub static UPTIME_SECS: AtomicU32 = AtomicU32::new(0);

pub static HEARTBEATS: [AtomicU32; TaskId::COUNT as usize] = {
    const ZERO: AtomicU32 = AtomicU32::new(0);
    [ZERO; TaskId::COUNT as usize]
};

pub fn heartbeat(task_id: TaskId) {
    HEARTBEATS[task_id as usize].fetch_add(1, Ordering::Relaxed);
}

pub const MONITORED_TASKS: [TaskId; 3] = [TaskId::Audio, TaskId::IdleCounter, TaskId::StatsTask];

pub struct RamStats {
    pub data_bytes: usize,
    pub bss_bytes: usize,
    pub static_used: usize,
    pub total_ram: usize,
}

impl Clone for RamStats {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for RamStats {}

extern "C" {
    static __sdata: u8;
    static __edata: u8;
    static __sbss: u8;
    static __ebss: u8;
}

pub fn ram_stats() -> RamStats {
    let data_bytes = unsafe { &__edata as *const u8 as usize - &__sdata as *const u8 as usize };
    let bss_bytes = unsafe { &__ebss as *const u8 as usize - &__sbss as *const u8 as usize };
    RamStats {
        data_bytes,
        bss_bytes,
        static_used: data_bytes + bss_bytes,
        total_ram: 640 * 1024,
    }
}
