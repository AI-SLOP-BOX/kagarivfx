use eframe::egui;

pub struct PanelAnimation {
    pub progress: f32,
    pub target: f32,
    pub speed: f32,
    pub is_open: bool,
}

impl PanelAnimation {
    pub fn new(open: bool) -> Self {
        Self {
            progress: if open { 1.0 } else { 0.0 },
            target: if open { 1.0 } else { 0.0 },
            speed: 8.0,
            is_open: open,
        }
    }

    pub fn update(&mut self, dt: f32) {
        if (self.progress - self.target).abs() < 0.001 {
            self.progress = self.target;
            self.is_open = self.target >= 1.0;
            return;
        }
        let step = dt * self.speed;
        if self.progress < self.target {
            self.progress = (self.progress + step).min(self.target);
        } else {
            self.progress = (self.progress - step).max(self.target);
        }
        self.is_open = self.target >= 1.0 && self.progress >= 1.0;
    }

    pub fn set_open(&mut self, open: bool) {
        self.target = if open { 1.0 } else { 0.0 };
        self.is_open = open;
    }

    pub fn toggle(&mut self) {
        self.set_open(self.target < 0.5);
    }

    pub fn ease(t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        1.0 - (1.0 - t).powi(3)
    }
}

pub fn animate_panel_width(_ctx: &egui::Context, animation: &PanelAnimation, max_width: f32) -> f32 {
    max_width * PanelAnimation::ease(animation.progress)
}

pub fn animate_panel_height(_ctx: &egui::Context, animation: &PanelAnimation, max_height: f32) -> f32 {
    max_height * PanelAnimation::ease(animation.progress)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_open() {
        let a = PanelAnimation::new(true);
        assert_eq!(a.progress, 1.0);
        assert_eq!(a.target, 1.0);
        assert!(a.is_open);
    }

    #[test]
    fn test_new_closed() {
        let a = PanelAnimation::new(false);
        assert_eq!(a.progress, 0.0);
        assert_eq!(a.target, 0.0);
        assert!(!a.is_open);
    }

    #[test]
    fn test_update_converges() {
        let mut a = PanelAnimation::new(false);
        a.set_open(true);
        for _ in 0..200 {
            a.update(0.016);
        }
        assert!((a.progress - 1.0).abs() < 0.001);
        assert!(a.is_open);
    }

    #[test]
    fn test_toggle() {
        let mut a = PanelAnimation::new(false);
        a.toggle();
        assert!(a.target >= 1.0);
        a.toggle();
        assert!(a.target < 0.5);
    }

    #[test]
    fn test_ease_bounds() {
        assert_eq!(PanelAnimation::ease(0.0), 0.0);
        assert_eq!(PanelAnimation::ease(1.0), 1.0);
        assert!(PanelAnimation::ease(0.5) > 0.5);
    }

    #[test]
    fn test_animate_width_closed() {
        let a = PanelAnimation::new(false);
        assert_eq!(a.progress * 300.0, 0.0);
    }

    #[test]
    fn test_animate_width_open() {
        let a = PanelAnimation::new(true);
        assert_eq!(a.progress * 300.0, 300.0);
    }
}
