use std::f32::consts::{PI, TAU};

use bevy::prelude::Vec3;

#[derive(Debug, Default)]
pub struct SecondOrderDynamics {
    // Previous input.
    xp: Vec3,
    // State variables.
    y: Vec3,
    yd: Vec3,
    // Computed constants.
    w: f32,
    z: f32,
    d: f32,
    k1: f32,
    k2: f32,
    k3: f32,
}

impl SecondOrderDynamics {
    pub fn new(f: f32, z: f32, r: f32, x0: Vec3) -> Self {
        let w = TAU * f;
        let d = w * (z * z - 1.0).abs().sqrt();

        Self {
            w,
            z,
            d,
            k1: z / (PI * f),
            k2: 1.0 / (w * w),
            k3: r * z / w,
            xp: x0,
            y: x0,
            yd: Vec3::ZERO,
        }
    }

    pub fn update(&mut self, t: f32, x: Vec3, xd: Option<Vec3>) -> Vec3 {
        // estimate velocity
        let xd = xd.unwrap_or_else(|| {
            assert!(t != 0.0);
            let xd = (x - self.xp) / t;
            self.xp = x;
            xd
        });

        // compute stable k1/k2
        let (k1, k2) = if self.w * t < self.z {
            // clamp k2 to guarantee stability without jitter
            (
                self.k1,
                self.k2
                    .max(t * t / 2.0 + t * self.k1 / 2.0)
                    .max(t * self.k1),
            )
        } else {
            // use pole matching when the system is very fast
            let t1 = (-self.z * self.w * t).exp();
            let alpha = 2.0
                * t1
                * if self.z <= 1.0 {
                    (t * self.d).cos()
                } else {
                    (t * self.d).cosh()
                };
            let beta = t1 * t1;
            let t2 = t / (1.0 + beta - alpha);
            ((1.0 - beta) * t2, t * t2)
        };

        // integrate position by velocity
        self.y = self.y + t * self.yd;

        // integrate velocity by acceleration
        self.yd += t * (x + self.k3 * xd - self.y - k1 * self.yd) / k2;

        self.y
    }
}

#[cfg(test)]
mod tests {
    use super::SecondOrderDynamics;
    use bevy::prelude::Vec3;

    #[test]
    fn it_works() {
        let mut dynamics = SecondOrderDynamics::new(1.0, 1.0, 1.0, Vec3::ZERO);
        let _ = dynamics.update(0.01, Vec3::X, Some(Vec3::X));
        let t = dynamics.update(0.01, Vec3::X, Some(Vec3::X));
        assert_ne!(t, Vec3::ZERO);
    }
}
