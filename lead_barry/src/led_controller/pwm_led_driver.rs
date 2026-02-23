use defmt_or_log as log;
use embassy_rp::Peri;
use embassy_rp::peripherals::{PIN_18, PIN_19, PIN_22, PWM_SLICE1, PWM_SLICE3};
use embassy_rp::pwm::{Config, Pwm, PwmError, PwmOutput, SetDutyCycle};

const PWM_FREQUENCY_HZ: u32 = 5_000;
const PWM_DIVIDER: u8 = 16;

pub type LedError = PwmError;

#[allow(dead_code)]
#[repr(usize)]
#[derive(Debug, Clone, Copy)]
pub enum PwmLed {
    Red = 0,
    Yellow = 1,
    Blue = 2,
}

impl TryFrom<usize> for PwmLed {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(PwmLed::Red),
            1 => Ok(PwmLed::Yellow),
            2 => Ok(PwmLed::Blue),
            _ => Err(()),
        }
    }
}

const LED_COUNT: usize = PwmLed::Blue as usize + 1;

pub struct PwmHardwareConfig {
    pub slice1: Peri<'static, PWM_SLICE1>,
    pub slice3: Peri<'static, PWM_SLICE3>,

    pub led_red: Peri<'static, PIN_18>,
    pub led_yellow: Peri<'static, PIN_19>,
    pub led_blue: Peri<'static, PIN_22>,
}

pub struct PwmLedDriver {
    leds: [PwmOutput<'static>; LED_COUNT],
}

#[allow(dead_code)]
impl PwmLedDriver {
    pub fn new(config: PwmHardwareConfig) -> Self {
        let desired_freq_hz = PWM_FREQUENCY_HZ;
        let clock_freq_hz = embassy_rp::clocks::clk_sys_freq();
        let divider = PWM_DIVIDER;
        let period = (clock_freq_hz / (desired_freq_hz * divider as u32)) as u16 - 1;

        log::debug!(
            "PwmLedDriver: Configuring PWM: clock_freq={} Hz, desired_freq={} Hz, divider={}, period={}",
            clock_freq_hz,
            desired_freq_hz,
            divider,
            period
        );

        let mut pwm_config = Config::default();
        pwm_config.top = period;
        pwm_config.divider = divider.into();

        let pwm1 = Pwm::new_output_ab(config.slice1, config.led_red, config.led_yellow, pwm_config.clone());
        let pwm2 = Pwm::new_output_a(config.slice3, config.led_blue, pwm_config.clone());

        let (pwm_red_opt, pwm_yellow_opt) = pwm1.split();
        let (pwm_blue_opt, _) = pwm2.split();

        let mut led_red_pwm = pwm_red_opt.unwrap();
        let mut led_yellow_pwm = pwm_yellow_opt.unwrap();
        let mut led_blue_pwm = pwm_blue_opt.unwrap();

        led_red_pwm.set_duty_cycle_fully_off().unwrap();
        led_yellow_pwm.set_duty_cycle_fully_off().unwrap();
        led_blue_pwm.set_duty_cycle_fully_off().unwrap();

        Self {
            leds: [led_red_pwm, led_yellow_pwm, led_blue_pwm],
        }
    }

    #[inline]
    pub fn set_intensity_percent(&mut self, led: PwmLed, percent: u8) -> Result<(), LedError> {
        self.leds[led as usize].set_duty_cycle_percent(percent)
    }

    pub fn set_intensity_fraction(&mut self, led: PwmLed, num: u16, denom: u16) -> Result<(), LedError> {
        self.leds[led as usize].set_duty_cycle_fraction(num, denom)
    }

    #[inline]
    pub fn led_on(&mut self, led: PwmLed) -> Result<(), LedError> {
        self.leds[led as usize].set_duty_cycle_fully_on()
    }

    #[inline]
    pub fn led_off(&mut self, led: PwmLed) -> Result<(), LedError> {
        self.leds[led as usize].set_duty_cycle_fully_off()
    }
}
