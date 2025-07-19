pub mod render;

#[derive(Debug)]
pub struct ViewDirection {
    pub center: (i32, i32),
    pub yaw: Yaw,
    pub pitch: Pitch,
    fov: u32,
}

impl ViewDirection {
    pub fn new(size: (i32, i32)) -> Self {
        Self {
            center: (size.0 / 2, size.1 / 2),
            yaw: Yaw::default(),
            pitch: Pitch::default(),
            fov: 90,
        }
    }

    pub fn resize(&mut self, new: (i32, i32)) {
        self.center = (new.0 / 2, new.1 / 2);
    }

    pub fn update(&mut self, new: (i32, i32)) {
        self.yaw.rot((new.0 - self.center.0) / 8);
        self.pitch.rot((self.center.1 - new.1) / 8);
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Yaw(i32);

impl Yaw {
    pub fn rot(&mut self, amt: i32) {
        self.0 += amt;
        if self.0 < -360 || self.0 > 360 {
            self.0 = self.0 % 360;
        }
        if self.0 < 0 {
            self.0 += 360;
        }
    }
}

impl Default for Yaw {
    fn default() -> Self {
        Self(0)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Pitch(i32);

impl Pitch {
    pub fn rot(&mut self, amt: i32) {
        self.0 += amt;
        if self.0 > 180 {
            self.0 = 180;
        } else if self.0 < 0 {
            self.0 = 0;
        }
    }
}

impl Default for Pitch {
    fn default() -> Self {
        Self(90)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn inc_pitch() {
        let mut pitch = Pitch::default();
        pitch.rot(60);
        assert_eq!(pitch.0, 150);
        pitch.rot(60);
        assert_eq!(pitch.0, 180);
    }

    #[test]
    fn dec_pitch() {
        let mut pitch = Pitch::default();
        pitch.rot(-60);
        assert_eq!(pitch.0, 30);
        pitch.rot(-60);
        assert_eq!(pitch.0, 0);
    }

    #[test]
    fn inc_yaw() {
        let mut yaw = Yaw::default();
        yaw.rot(220);
        assert_eq!(yaw.0, 220);
        yaw.rot(200);
        assert_eq!(yaw.0, 60);
    }

    #[test]
    fn dec_yaw() {
        let mut pitch = Yaw::default();
        pitch.rot(-220);
        assert_eq!(pitch.0, 140);
        pitch.rot(-20);
        assert_eq!(pitch.0, 120);
    }
}
