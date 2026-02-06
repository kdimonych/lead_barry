mod constants;
mod decay_animation;
mod pwm_led_driver;
mod sine_animation;

use constants::*;
use decay_animation::DecayAnimation;
use postcard::fixint::le;
pub use pwm_led_driver::PwmHardwareConfig;
use pwm_led_driver::PwmLedDriver;
use sine_animation::SineAnimation;

use defmt as log;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Receiver, Sender};
use embassy_time::Ticker;
use static_cell::StaticCell;

type LedMessageChanel = Channel<CriticalSectionRawMutex, LedMessage, MAX_MESSAGE_QUEUE_SIZE>;
type LedMessageSender =
    Sender<'static, CriticalSectionRawMutex, LedMessage, MAX_MESSAGE_QUEUE_SIZE>;
type LedMessageReceiver =
    Receiver<'static, CriticalSectionRawMutex, LedMessage, MAX_MESSAGE_QUEUE_SIZE>;

static LED_CONTROLLER_STATE: StaticCell<State> = StaticCell::new();

pub use pwm_led_driver::PwmLed as Led;

///Period in milliseconds (0-65535) for LED animations
pub type PeriodMs = u16;

/// Number of repetitions for LED animations
/// - Once: Play once
/// - Infinite: Repeat indefinitely
/// - Finite(u8): Repeat a finite number of times
#[allow(dead_code)]
#[derive(defmt::Format, Clone, Copy)]
pub enum Repetitions {
    Infinite,
    Finite(u8),
}

#[allow(dead_code)]
#[derive(defmt::Format, Clone, Copy)]
pub enum LedAnimation {
    /// Turn on the LED at full brightnes
    On,
    /// Turn on the LED at full brightness and then decay gradually to off with the given period
    Decay(PeriodMs),
    // /// Breathe the LED in and out for the given period and repetitions
    // Heartbeat(PeriodMs, Repetitions),
    /// Animate the LED with a sine wave pattern for the given period and repetitions
    Sine(PeriodMs, Repetitions),
    /// Blink the LED on and off with the given period and repetitions
    Blinks(PeriodMs, Repetitions),
    /// Blink the LED in a pattern that indicates an alert (e.g. fast blinks)
    Alert,
    /// Turn off the LED
    Off,
}

#[derive(Clone, Copy)]
struct LedMessage {
    led: Led,
    animation: LedAnimation,
}

struct State {
    channel: LedMessageChanel,
}

pub struct LedControllerRunner {
    hardware_config: PwmHardwareConfig,
    receiver: LedMessageReceiver,
}

pub struct LedControllerBuilder {
    hardware_config: PwmHardwareConfig,
    receiver: LedMessageReceiver,
    sender: LedMessageSender,
}

impl LedControllerBuilder {
    pub fn new(hardware_config: PwmHardwareConfig) -> Self {
        let channel = Channel::new();
        let state = LED_CONTROLLER_STATE.init(State { channel });

        Self {
            hardware_config,
            receiver: state.channel.receiver(),
            sender: state.channel.sender(),
        }
    }

    pub fn build_once(self) -> (LedController, LedControllerRunner) {
        let runner = LedControllerRunner {
            hardware_config: self.hardware_config,
            receiver: self.receiver,
        };

        let controller = LedController {
            sender: self.sender,
        };

        (controller, runner)
    }
}

#[derive(Clone, Copy)]
pub struct LedController {
    sender: LedMessageSender,
}

impl LedController {
    #[inline]
    pub fn try_set_animation(&self, led: Led, animation: LedAnimation) -> Result<(), ()> {
        self.sender
            .try_send(LedMessage { led, animation })
            .map_err(|_| ())
    }

    pub async fn set_animation(&self, led: Led, animation: LedAnimation) {
        self.sender.send(LedMessage { led, animation }).await;
    }
}

enum Animator {
    None,
    Sine(SineAnimation),
    Decay(DecayAnimation),
    // Other animation types can be added here
}
impl Animator {
    fn next_sample(&mut self) -> Option<u16> {
        match self {
            Animator::None => None,
            Animator::Sine(anim) => anim.next(),
            Animator::Decay(anim) => anim.next(),
        }
    }
}

impl LedControllerRunner {
    pub async fn run(self) -> ! {
        let mut active_animator: [Animator; 3] = core::array::from_fn(|_| Animator::None);

        let mut led_driver = PwmLedDriver::new(self.hardware_config);
        let mut ticker = Ticker::every(DELTA_T);
        loop {
            if let Ok(message) = self.receiver.try_receive() {
                match message.animation {
                    LedAnimation::On => led_driver.led_on(message.led).unwrap(),
                    LedAnimation::Off => led_driver.led_off(message.led).unwrap(),
                    LedAnimation::Sine(period_ms, repetitions) => {
                        let mut infinite = false;
                        let base_animation_period = SAMPLE_RATE * period_ms as u32 / 1000;
                        let animation_period = match repetitions {
                            Repetitions::Infinite => {
                                infinite = true;
                                base_animation_period
                            }
                            Repetitions::Finite(n) => base_animation_period * n as u32,
                        };

                        active_animator[message.led as usize] = Animator::Sine(SineAnimation::new(
                            animation_period,
                            MAGNITUDE,
                            1,
                            infinite,
                        ));
                    }
                    LedAnimation::Decay(period_ms) => {
                        let animation_period = (SAMPLE_RATE * period_ms as u32) / 1000 + 1;

                        active_animator[message.led as usize] =
                            Animator::Decay(DecayAnimation::new(animation_period, MAGNITUDE));
                    }

                    _ => {
                        // For simplicity, other animations are not implemented in this snippet
                        log::warn!("Animation {:?} not implemented yet", message.animation);
                        active_animator[message.led as usize] = Animator::None;
                    }
                };
            };

            for (animator_idx, animator) in &mut active_animator.iter_mut().enumerate() {
                if let Some(sample) = animator.next_sample() {
                    led_driver
                        .set_intensity_fraction(animator_idx.try_into().unwrap(), sample, MAGNITUDE)
                        .unwrap();
                } else {
                    // Animation finished, disable animator
                    led_driver
                        .led_off(animator_idx.try_into().unwrap())
                        .unwrap();
                    *animator = Animator::None;
                }
            }

            ticker.next().await;
        }
    }
}
