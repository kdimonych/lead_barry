pub mod color;
mod constants;
mod decay_animation;
mod sine_animation;
mod ws2812b_led_driver;

use constants::*;
use decay_animation::DecayAnimation;
use embassy_rp::Peri;
use embassy_rp::dma::Channel as DmaChannel;
use embassy_rp::pio::{Common, Instance, PioPin, StateMachine};
use embassy_rp::pio_programs::ws2812::{PioWs2812, PioWs2812Program};
use sine_animation::SineAnimation;
use ws2812b_led_driver::Ws2812LedDriver;

use defmt_or_log as log;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Receiver, Sender};
use embassy_time::Ticker;
use static_cell::StaticCell;

type LedMessageChanel = Channel<CriticalSectionRawMutex, LedMessage, MAX_MESSAGE_QUEUE_SIZE>;
type LedMessageSender = Sender<'static, CriticalSectionRawMutex, LedMessage, MAX_MESSAGE_QUEUE_SIZE>;
type LedMessageReceiver = Receiver<'static, CriticalSectionRawMutex, LedMessage, MAX_MESSAGE_QUEUE_SIZE>;

static LED_CONTROLLER_STATE: StaticCell<State> = StaticCell::new();

use color::Color;
pub use ws2812b_led_driver::Led;

///Period in milliseconds (0-65535) for LED animations
pub type PeriodMs = u16;

/// Number of repetitions for LED animations
/// - Once: Play once
/// - Infinite: Repeat indefinitely
/// - Finite(u8): Repeat a finite number of times
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum Repetitions {
    Infinite,
    Finite(u8),
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum LedAnimation {
    /// Turn on the LED at full brightness
    On(Color),
    /// Turn on the LED at full brightness and then decay gradually to off with the given period
    Decay(Color, PeriodMs),
    // Heartbeat(PeriodMs, Repetitions),
    /// Animate the LED with a sine wave pattern for the given period and repetitions
    Sine(Color, PeriodMs, Repetitions),
    /// Blink the LED on and off with the given period and repetitions
    Blinks(Color, PeriodMs, Repetitions),
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

pub struct LedControllerRunner<'d, P: Instance, const S: usize, const N: usize> {
    driver: Ws2812LedDriver<'d, P, S, N>,
    receiver: LedMessageReceiver,
}

pub struct LedControllerBuilder<const N: usize> {
    receiver: LedMessageReceiver,
    sender: LedMessageSender,
}

impl<const N: usize> LedControllerBuilder<N> {
    pub fn new() -> Self {
        let channel = Channel::new();
        let state = LED_CONTROLLER_STATE.init(State { channel });

        Self {
            receiver: state.channel.receiver(),
            sender: state.channel.sender(),
        }
    }

    pub fn build_once<'d, P: Instance, const S: usize>(
        self,
        pio: &mut Common<'d, P>,
        sm: StateMachine<'d, P, S>,
        dma: Peri<'d, impl DmaChannel>,
        pin: Peri<'d, impl PioPin>,
    ) -> (LedController, LedControllerRunner<'d, P, S, N>) {
        let program = PioWs2812Program::new(pio);
        let ws2812 = PioWs2812::new(pio, sm, dma, pin, &program);

        let runner = LedControllerRunner {
            driver: Ws2812LedDriver::new(ws2812),
            receiver: self.receiver,
        };

        let controller = LedController { sender: self.sender };

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
        self.sender.try_send(LedMessage { led, animation }).map_err(|_| ())
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
    fn next_sample(&mut self) -> Option<Color> {
        match self {
            Animator::None => None,
            Animator::Sine(anim) => anim.next(),
            Animator::Decay(anim) => anim.next(),
        }
    }
}

impl<'d, P: Instance, const S: usize, const N: usize> LedControllerRunner<'d, P, S, N> {
    async fn read_message(&mut self, active_animator: &mut [Animator; N]) {
        if let Ok(message) = self.receiver.try_receive() {
            match message.animation {
                LedAnimation::On(color) => self.driver.led_on(message.led, color),
                LedAnimation::Off => self.driver.led_off(message.led),
                LedAnimation::Sine(color, period_ms, repetitions) => {
                    let mut infinite = false;
                    let base_animation_period = SAMPLE_RATE * period_ms as u32 / 1000;
                    let animation_period = match repetitions {
                        Repetitions::Infinite => {
                            infinite = true;
                            base_animation_period
                        }
                        Repetitions::Finite(n) => base_animation_period * n as u32,
                    };

                    active_animator[message.led as usize] =
                        Animator::Sine(SineAnimation::new(color, animation_period, 1, infinite));
                }
                LedAnimation::Decay(color, period_ms) => {
                    let animation_period = (SAMPLE_RATE * period_ms as u32) / 1000 + 1;

                    active_animator[message.led as usize] =
                        Animator::Decay(DecayAnimation::new(color, animation_period));
                }

                _ => {
                    // For simplicity, other animations are not implemented in this snippet
                    log::warn!(
                        "Animation {:?} not implemented yet",
                        defmt_or_log::Debug2Format(&message.animation)
                    );
                    active_animator[message.led as usize] = Animator::None;
                }
            };
        };
    }

    pub async fn run(mut self) -> ! {
        let mut active_animator: [Animator; N] = core::array::from_fn(|_| Animator::None);

        let mut ticker = Ticker::every(DELTA_T);
        loop {
            self.read_message(&mut active_animator).await;

            let mut do_flush = false;

            for (animator_idx, animator) in &mut active_animator
                .iter_mut()
                .enumerate()
                .filter(|(_, a)| !matches!(a, Animator::None))
            {
                if let Some(sample) = animator.next_sample() {
                    self.driver.set_color(animator_idx, sample);
                } else {
                    // Animation finished, disable animator
                    self.driver.led_off(animator_idx);
                    *animator = Animator::None;
                }
                do_flush = true;
            }

            if do_flush {
                self.driver.flush().await;
            }

            ticker.next().await;
        }
    }
}
