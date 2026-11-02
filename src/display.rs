//! Picks the monitor the game should fill.
//!
//! The game sizes its window from the primary display, and dgVoodoo presents
//! on the adapter's default output. With a portrait primary monitor that
//! squeezes the 16:9 picture into a strip, so a landscape monitor is chosen
//! and handed to dgVoodoo as `FullScreenOutput`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Monitor {
    pub width: u32,
    pub height: u32,
    pub primary: bool,
}

impl Monitor {
    fn landscape(self) -> bool {
        self.width >= self.height
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    pub size: (u32, u32),
    /// 1-based dgVoodoo `FullScreenOutput`, or `None` to keep its default.
    pub output: Option<u32>,
}

/// Keeps the primary monitor when it is landscape. Otherwise uses the
/// largest landscape monitor, numbered in enumeration order.
pub fn choose(monitors: &[Monitor]) -> Option<Target> {
    let primary = monitors.iter().find(|m| m.primary).or(monitors.first())?;
    if primary.landscape() {
        return Some(Target {
            size: (primary.width, primary.height),
            output: None,
        });
    }
    let best = monitors
        .iter()
        .enumerate()
        .filter(|(_, m)| m.landscape())
        .max_by_key(|(index, m)| (u64::from(m.width) * u64::from(m.height), usize::MAX - index));
    Some(match best {
        Some((index, m)) => Target {
            size: (m.width, m.height),
            output: Some(index as u32 + 1),
        },
        None => Target {
            size: (primary.width, primary.height),
            output: None,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(width: u32, height: u32, primary: bool) -> Monitor {
        Monitor {
            width,
            height,
            primary,
        }
    }

    #[test]
    fn landscape_primary_keeps_dgvoodoo_default() {
        let target = choose(&[m(2160, 3840, false), m(3840, 2160, true)]).unwrap();
        assert_eq!(target.size, (3840, 2160));
        assert_eq!(target.output, None);
    }

    #[test]
    fn portrait_primary_selects_the_landscape_monitor() {
        // Measured layout: Wine lists the rotated DP-2 first as primary.
        let target = choose(&[m(2160, 3840, true), m(3840, 2160, false)]).unwrap();
        assert_eq!(target.size, (3840, 2160));
        assert_eq!(target.output, Some(2));
    }

    #[test]
    fn largest_landscape_wins_and_ties_keep_the_first() {
        let monitors = [
            m(1080, 1920, true),
            m(1920, 1080, false),
            m(2560, 1440, false),
            m(2560, 1440, false),
        ];
        assert_eq!(choose(&monitors).unwrap().output, Some(3));
    }

    #[test]
    fn only_portrait_or_nothing_falls_back() {
        let target = choose(&[m(1080, 1920, true)]).unwrap();
        assert_eq!(target.size, (1080, 1920));
        assert_eq!(target.output, None);
        assert_eq!(choose(&[]), None);
    }
}
