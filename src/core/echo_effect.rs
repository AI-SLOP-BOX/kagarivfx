//! Echo Time Effect Engine (AE Parity).
//!
//! Evaluates multiple frames across a time window (past or future) and composites
//! them together with intensity decay using standard After Effects echo operators
//! (Add, Screen, Maximum, Minimum, Composite in Back, Composite in Front, Blend).

#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum EchoOperator {
    #[default]
    Add,
    Screen,
    Maximum,
    Minimum,
    CompositeInBack,
    CompositeInFront,
    Blend,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EchoParams {
    pub echo_time_seconds: f32, // Offset between echoes (e.g. -0.033 = 1 frame back at 30fps)
    pub num_echoes: u32,        // 1..30
    pub starting_intensity: f32,// 0.0..2.0
    pub decay: f32,             // 0.0..1.0 multiplier per echo
    pub operator: EchoOperator,
}

impl Default for EchoParams {
    fn default() -> Self {
        Self {
            echo_time_seconds: -0.033,
            num_echoes: 3,
            starting_intensity: 1.0,
            decay: 0.5,
            operator: EchoOperator::Add,
        }
    }
}

/// Blends an echo frame into the accumulator buffer according to the chosen EchoOperator.
pub fn blend_echo_frame(
    acc: &mut [u8],
    echo: &[u8],
    width: u32,
    height: u32,
    weight: f32,
    op: EchoOperator,
) {
    if acc.len() != (width * height * 4) as usize || echo.len() != acc.len() {
        return;
    }

    let w = weight.clamp(0.0, 2.0);

    for idx in (0..acc.len()).step_by(4) {
        let ar = acc[idx] as f32 / 255.0;
        let ag = acc[idx + 1] as f32 / 255.0;
        let ab = acc[idx + 2] as f32 / 255.0;
        let aa = acc[idx + 3] as f32 / 255.0;

        let er = (echo[idx] as f32 / 255.0) * w;
        let eg = (echo[idx + 1] as f32 / 255.0) * w;
        let eb = (echo[idx + 2] as f32 / 255.0) * w;
        let ea = (echo[idx + 3] as f32 / 255.0) * w.min(1.0);

        let (nr, ng, nb, na) = match op {
            EchoOperator::Add => (
                (ar + er).min(1.0),
                (ag + eg).min(1.0),
                (ab + eb).min(1.0),
                (aa + ea).min(1.0),
            ),
            EchoOperator::Screen => (
                1.0 - (1.0 - ar) * (1.0 - er.min(1.0)),
                1.0 - (1.0 - ag) * (1.0 - eg.min(1.0)),
                1.0 - (1.0 - ab) * (1.0 - eb.min(1.0)),
                1.0 - (1.0 - aa) * (1.0 - ea.min(1.0)),
            ),
            EchoOperator::Maximum => (
                ar.max(er),
                ag.max(eg),
                ab.max(eb),
                aa.max(ea),
            ),
            EchoOperator::Minimum => (
                ar.min(er),
                ag.min(eg),
                ab.min(eb),
                aa.min(ea),
            ),
            EchoOperator::CompositeInBack => {
                let out_a = ea + aa * (1.0 - ea);
                if out_a <= 0.0 {
                    (0.0, 0.0, 0.0, 0.0)
                } else {
                    let out_r = (er * ea + ar * aa * (1.0 - ea)) / out_a;
                    let out_g = (eg * ea + ag * aa * (1.0 - ea)) / out_a;
                    let out_b = (eb * ea + ab * aa * (1.0 - ea)) / out_a;
                    (out_r, out_g, out_b, out_a)
                }
            }
            EchoOperator::CompositeInFront => {
                let out_a = aa + ea * (1.0 - aa);
                if out_a <= 0.0 {
                    (0.0, 0.0, 0.0, 0.0)
                } else {
                    let out_r = (ar * aa + er * ea * (1.0 - aa)) / out_a;
                    let out_g = (ag * aa + eg * ea * (1.0 - aa)) / out_a;
                    let out_b = (ab * aa + eb * ea * (1.0 - aa)) / out_a;
                    (out_r, out_g, out_b, out_a)
                }
            }
            EchoOperator::Blend => (
                (ar + er) * 0.5,
                (ag + eg) * 0.5,
                (ab + eb) * 0.5,
                (aa + ea) * 0.5,
            ),
        };

        acc[idx] = (nr.clamp(0.0, 1.0) * 255.0).round() as u8;
        acc[idx + 1] = (ng.clamp(0.0, 1.0) * 255.0).round() as u8;
        acc[idx + 2] = (nb.clamp(0.0, 1.0) * 255.0).round() as u8;
        acc[idx + 3] = (na.clamp(0.0, 1.0) * 255.0).round() as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_echo_add_operator() {
        let mut acc = vec![100, 100, 100, 255];
        let echo = vec![100, 100, 100, 255];

        blend_echo_frame(&mut acc, &echo, 1, 1, 1.0, EchoOperator::Add);
        assert_eq!(acc[0], 200);
        assert_eq!(acc[3], 255);
    }

    #[test]
    fn test_echo_screen_operator() {
        let mut acc = vec![128, 128, 128, 255];
        let echo = vec![128, 128, 128, 255];

        blend_echo_frame(&mut acc, &echo, 1, 1, 1.0, EchoOperator::Screen);
        assert!(acc[0] > 128);
    }
}
