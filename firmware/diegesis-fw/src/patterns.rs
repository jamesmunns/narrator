use crate::groundhog_nrf52::GlobalRollingTimer;
use choreographer::engine::{Behavior, Sequence};
use choreographer::script;
use smart_leds::RGB8;

pub fn rainbow_crawler<const LEDS: usize, const MAX_STEPS: usize>(
    script: &mut [Sequence<GlobalRollingTimer, MAX_STEPS>; LEDS],
    behavior: Behavior,
    direction: Direction,
) {
    let plan = &[
        script! {
            | action |  color | duration_ms | period_ms_f | phase_offset_ms | repeat |
            |  solid |  BLACK |           0 |         0.0 |               0 |   once |
            |    sin |  WHITE |        2500 |      2500.0 |               0 |   once |
            |  solid |  BLACK |        1000 |         0.0 |               0 |   once |
        },
        script! {
            | action |  color | duration_ms | period_ms_f | phase_offset_ms | repeat |
            |  solid |  BLACK |         100 |         0.0 |               0 |   once |
            |    sin |  WHITE |        2500 |      2500.0 |               0 |   once |
            |  solid |  BLACK |         900 |         0.0 |               0 |   once |
        },
        script! {
            | action |  color | duration_ms | period_ms_f | phase_offset_ms | repeat |
            |  solid |  BLACK |         200 |         0.0 |               0 |   once |
            |    sin |    RED |        2500 |      2500.0 |               0 |   once |
            |  solid |  BLACK |         800 |         0.0 |               0 |   once |
        },
        script! {
            | action |  color | duration_ms | period_ms_f | phase_offset_ms | repeat |
            |  solid |  BLACK |         300 |         0.0 |               0 |   once |
            |    sin | ORANGE |        2500 |      2500.0 |               0 |   once |
            |  solid |  BLACK |         700 |         0.0 |               0 |   once |
        },
        script! {
            | action |  color | duration_ms | period_ms_f | phase_offset_ms | repeat |
            |  solid |  BLACK |         400 |         0.0 |               0 |   once |
            |    sin | YELLOW |        2500 |      2500.0 |               0 |   once |
            |  solid |  BLACK |         600 |         0.0 |               0 |   once |
        },
        script! {
            | action |  color | duration_ms | period_ms_f | phase_offset_ms | repeat |
            |  solid |  BLACK |         500 |         0.0 |               0 |   once |
            |    sin |  GREEN |        2500 |      2500.0 |               0 |   once |
            |  solid |  BLACK |         500 |         0.0 |               0 |   once |
        },
        script! {
            | action |  color | duration_ms | period_ms_f | phase_offset_ms | repeat |
            |  solid |  BLACK |         600 |         0.0 |               0 |   once |
            |    sin |   BLUE |        2500 |      2500.0 |               0 |   once |
            |  solid |  BLACK |         400 |         0.0 |               0 |   once |
        },
        script! {
            | action |  color | duration_ms | period_ms_f | phase_offset_ms | repeat |
            |  solid |  BLACK |         700 |         0.0 |               0 |   once |
            |    sin | VIOLET |        2500 |      2500.0 |               0 |   once |
            |  solid |  BLACK |         300 |         0.0 |               0 |   once |
        },
        script! {
            | action |  color | duration_ms | period_ms_f | phase_offset_ms | repeat |
            |  solid |  BLACK |         800 |         0.0 |               0 |   once |
            |    sin |  WHITE |        2500 |      2500.0 |               0 |   once |
            |  solid |  BLACK |         200 |         0.0 |               0 |   once |
        },
        script! {
            | action |  color | duration_ms | period_ms_f | phase_offset_ms | repeat |
            |  solid |  BLACK |         900 |         0.0 |               0 |   once |
            |    sin |  WHITE |        2500 |      2500.0 |               0 |   once |
            |  solid |  BLACK |         100 |         0.0 |               0 |   once |
        }
    ];

    let cw: &mut dyn Iterator<Item = _> = &mut plan.iter().rev();
    let ccw: &mut dyn Iterator<Item = _> = &mut plan.iter();

    let plan = match direction {
        Direction::Clockwise => cw,
        Direction::CounterClockwise => ccw,
    };

    script
        .iter_mut()
        .zip(plan)
        .for_each(|(scr, pla)| {
            scr.set(pla, behavior.clone())
        })
}

trait ExtRgb {
    fn div(&self, div: u8) -> RGB8;
    fn mul(&self, mul: u8) -> RGB8;
    fn mul_then_div(&self, mul: u8, div: u8) -> RGB8;
}

impl ExtRgb for RGB8 {
    fn div(&self, div: u8) -> RGB8 {
        RGB8 {
            r: self.r / div,
            g: self.g / div,
            b: self.b / div,
        }
    }
    fn mul(&self, mul: u8) -> RGB8 {
        RGB8 {
            r: self.r * mul,
            g: self.g * mul,
            b: self.b * mul,
        }
    }

    fn mul_then_div(&self, mul: u8, div: u8) -> RGB8 {
        RGB8 {
            r: (((self.r as u16) * (mul as u16)) / (div as u16)) as u8,
            g: (((self.g as u16) * (mul as u16)) / (div as u16)) as u8,
            b: (((self.b as u16) * (mul as u16)) / (div as u16)) as u8,
        }
    }
}

pub fn color_walker<const LEDS: usize, const MAX_STEPS: usize>(
    script: &mut [Sequence<GlobalRollingTimer, MAX_STEPS>; LEDS],
    walk_color: RGB8,
    behavior: Behavior,
    direction: Direction,
) {
    let color_1_8 = walk_color.div(8);
    let color_2_8 = walk_color.mul_then_div(2, 8);
    let color_3_8 = walk_color.mul_then_div(3, 8);
    let color_4_8 = walk_color.mul_then_div(4, 8);
    let color_5_8 = walk_color.mul_then_div(5, 8);
    let color_6_8 = walk_color.mul_then_div(6, 8);
    let color_7_8 = walk_color.mul_then_div(7, 8);
    let color_8_8 = walk_color;

    let plan: &[[_; 8]] = &[
        script! {
            | action |      color | (duration_ms) | (period_ms_f) | (phase_offset_ms) |  repeat |
            |    cos |  color_1_8 | (        250) | (     2000.0) | (            250) |    once |
            |    cos |  color_2_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_3_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_4_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_5_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_6_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_7_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_8_8 | (       2000) | (     2000.0) | (       AutoIncr) | forever |
        },
        script! {
            | action |      color | (duration_ms) | (period_ms_f) | (phase_offset_ms) |  repeat |
            |    cos |  color_1_8 | (        250) | (     2000.0) | (            500) |    once |
            |    cos |  color_2_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_3_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_4_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_5_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_6_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_7_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_8_8 | (       2000) | (     2000.0) | (       AutoIncr) | forever |
        },
        script! {
            | action |      color | (duration_ms) | (period_ms_f) | (phase_offset_ms) |  repeat |
            |    cos |  color_1_8 | (        250) | (     2000.0) | (            750) |    once |
            |    cos |  color_2_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_3_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_4_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_5_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_6_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_7_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_8_8 | (       2000) | (     2000.0) | (       AutoIncr) | forever |
        },
        script! {
            | action |      color | (duration_ms) | (period_ms_f) | (phase_offset_ms) |  repeat |
            |    cos |  color_1_8 | (        250) | (     2000.0) | (           1000) |    once |
            |    cos |  color_2_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_3_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_4_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_5_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_6_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_7_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_8_8 | (       2000) | (     2000.0) | (       AutoIncr) | forever |
        },
        script! {
            | action |      color | (duration_ms) | (period_ms_f) | (phase_offset_ms) |  repeat |
            |    cos |  color_1_8 | (        250) | (     2000.0) | (           1250) |    once |
            |    cos |  color_2_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_3_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_4_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_5_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_6_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_7_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_8_8 | (       2000) | (     2000.0) | (       AutoIncr) | forever |
        },
        script! {
            | action |      color | (duration_ms) | (period_ms_f) | (phase_offset_ms) |  repeat |
            |    cos |  color_1_8 | (        250) | (     2000.0) | (           1500) |    once |
            |    cos |  color_2_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_3_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_4_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_5_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_6_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_7_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_8_8 | (       2000) | (     2000.0) | (       AutoIncr) | forever |
        },
        script! {
            | action |      color | (duration_ms) | (period_ms_f) | (phase_offset_ms) |  repeat |
            |    cos |  color_1_8 | (        250) | (     2000.0) | (           1750) |    once |
            |    cos |  color_2_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_3_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_4_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_5_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_6_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_7_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_8_8 | (       2000) | (     2000.0) | (       AutoIncr) | forever |
        },
        script! {
            | action |      color | (duration_ms) | (period_ms_f) | (phase_offset_ms) |  repeat |
            |    cos |  color_1_8 | (        250) | (     2000.0) | (           2000) |    once |
            |    cos |  color_2_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_3_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_4_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_5_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_6_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_7_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_8_8 | (       2000) | (     2000.0) | (       AutoIncr) | forever |
        },
        script! {
            | action |      color | (duration_ms) | (period_ms_f) | (phase_offset_ms) |  repeat |
            |    cos |  color_1_8 | (        250) | (     2000.0) | (           2250) |    once |
            |    cos |  color_2_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_3_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_4_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_5_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_6_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_7_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_8_8 | (       2000) | (     2000.0) | (       AutoIncr) | forever |
        },
        script! {
            | action |      color | (duration_ms) | (period_ms_f) | (phase_offset_ms) |  repeat |
            |    cos |  color_1_8 | (        250) | (     2000.0) | (           2500) |    once |
            |    cos |  color_2_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_3_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_4_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_5_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_6_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_7_8 | (        250) | (     2000.0) | (       AutoIncr) |    once |
            |    cos |  color_8_8 | (       2000) | (     2000.0) | (       AutoIncr) | forever |
        }
    ];

    let cw: &mut dyn Iterator<Item = _> = &mut plan.iter();
    let ccw: &mut dyn Iterator<Item = _> = &mut plan.iter().rev();

    let plan = match direction {
        Direction::Clockwise => cw,
        Direction::CounterClockwise => ccw,
    };

    script
        .iter_mut()
        .zip(plan)
        .for_each(|(scr, pla)| {
            scr.set(pla, behavior.clone())
        })
}

pub fn boot_seq<const LEDS: usize, const MAX_STEPS: usize>(
    script: &mut [Sequence<GlobalRollingTimer, MAX_STEPS>; LEDS],
    behavior: Behavior,
    _direction: Direction,
) {
    let mut delay = 0;

    script.iter_mut()
        .for_each(|scr| {
            let period_f = 2500.0 + (delay as f32) * 1.5;

            let plan = script! {
                | action  |   color | ( duration_ms) | (period_ms_f) | (phase_offset_ms) | repeat |
                |    sin  |    BLUE | (2500 + delay) | (   period_f) | (              0) |   once |
                |   seek  |     RED | (          50) | (        0.0) | (              0) |   once |
                |   seek  |  ORANGE | (          50) | (        0.0) | (              0) |   once |
                |   seek  |  YELLOW | (          50) | (        0.0) | (              0) |   once |
                |   seek  |   GREEN | (          50) | (        0.0) | (              0) |   once |
                |   seek  |    BLUE | (          50) | (        0.0) | (              0) |   once |
                |   seek  |  VIOLET | (          50) | (        0.0) | (              0) |   once |
                |   seek  |  BLACK  | (         100) | (        0.0) | (              0) |   once |
            };
            scr.set(&plan, behavior.clone());
            delay += 50;
        });
}

#[derive(Clone, Copy)]
pub enum Direction {
    Clockwise,
    CounterClockwise,
}

impl Direction {
    pub fn invert(&self) -> Self {
        match self {
            Direction::Clockwise => Direction::CounterClockwise,
            Direction::CounterClockwise => Direction::Clockwise,
        }
    }
}

