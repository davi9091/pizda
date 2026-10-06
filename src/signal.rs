/// A physical quantity the app knows about, independent of where it comes from.
///
/// Sources (ELM327, raw CAN, ADC, ...) all report values in terms of these,
/// so the UI and logger never need to know which input produced them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Signal {
    Rpm,
    SpeedKph,
    ThrottlePct,
    EngineLoadPct,
    CoolantTempC,
    IntakeAirTempC,
    TimingAdvanceDeg,
    MafGramsPerSec,
    ShortFuelTrimPct,
    LongFuelTrimPct,
    BatteryVolts,
}

impl Signal {
    /// Display order in the UI.
    pub const ALL: [Signal; 11] = [
        Signal::Rpm,
        Signal::SpeedKph,
        Signal::ThrottlePct,
        Signal::EngineLoadPct,
        Signal::CoolantTempC,
        Signal::IntakeAirTempC,
        Signal::TimingAdvanceDeg,
        Signal::MafGramsPerSec,
        Signal::ShortFuelTrimPct,
        Signal::LongFuelTrimPct,
        Signal::BatteryVolts,
    ];

    /// Stable identifier used in recordings. Never change an existing key,
    /// or old recordings stop loading.
    pub fn key(self) -> &'static str {
        match self {
            Signal::Rpm => "rpm",
            Signal::SpeedKph => "speed_kph",
            Signal::ThrottlePct => "throttle_pct",
            Signal::EngineLoadPct => "engine_load_pct",
            Signal::CoolantTempC => "coolant_temp_c",
            Signal::IntakeAirTempC => "intake_air_temp_c",
            Signal::TimingAdvanceDeg => "timing_advance_deg",
            Signal::MafGramsPerSec => "maf_g_s",
            Signal::ShortFuelTrimPct => "short_fuel_trim_pct",
            Signal::LongFuelTrimPct => "long_fuel_trim_pct",
            Signal::BatteryVolts => "battery_v",
        }
    }

    pub fn from_key(key: &str) -> Option<Signal> {
        Signal::ALL.into_iter().find(|s| s.key() == key)
    }

    pub fn label(self) -> &'static str {
        match self {
            Signal::Rpm => "Engine speed",
            Signal::SpeedKph => "Vehicle speed",
            Signal::ThrottlePct => "Throttle",
            Signal::EngineLoadPct => "Engine load",
            Signal::CoolantTempC => "Coolant temp",
            Signal::IntakeAirTempC => "Intake air temp",
            Signal::TimingAdvanceDeg => "Timing advance",
            Signal::MafGramsPerSec => "MAF",
            Signal::ShortFuelTrimPct => "Short fuel trim",
            Signal::LongFuelTrimPct => "Long fuel trim",
            Signal::BatteryVolts => "Battery",
        }
    }

    pub fn unit(self) -> &'static str {
        match self {
            Signal::Rpm => "rpm",
            Signal::SpeedKph => "km/h",
            Signal::ThrottlePct
            | Signal::EngineLoadPct
            | Signal::ShortFuelTrimPct
            | Signal::LongFuelTrimPct => "%",
            Signal::CoolantTempC | Signal::IntakeAirTempC => "°C",
            Signal::TimingAdvanceDeg => "°",
            Signal::MafGramsPerSec => "g/s",
            Signal::BatteryVolts => "V",
        }
    }

    /// Decimal places to show.
    pub fn precision(self) -> usize {
        match self {
            Signal::Rpm | Signal::SpeedKph | Signal::CoolantTempC | Signal::IntakeAirTempC => 0,
            Signal::BatteryVolts | Signal::MafGramsPerSec => 2,
            _ => 1,
        }
    }
}
