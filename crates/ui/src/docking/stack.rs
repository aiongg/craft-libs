//! The section layout of an accordion (`Node::Stack`): every section shows its title row, and only open sections
//! take height for their body.

/// One section of an accordion as the maths sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct StackSection {
    /// The section shows its body.
    pub open: bool,
    /// The preferred body height; `None` shares the free height.
    pub height: Option<f32>,
    /// The least body height of the section's panel.
    pub min: f32,
    /// The most body height of the section's panel.
    pub max: f32,
}

impl StackSection {
    /// A section whose body takes any height.
    pub const fn new(open: bool) -> Self {
        StackSection { open, height: None, min: 0.0, max: f32::INFINITY }
    }

    /// The section with a preferred body height.
    #[must_use]
    pub const fn height(mut self, height: Option<f32>) -> Self {
        self.height = height;
        self
    }

    /// The section with the body limits of its panel.
    #[must_use]
    pub const fn limits(mut self, min: f32, max: f32) -> Self {
        self.min = min;
        self.max = max;
        self
    }
}

/// The height of each section in `total` points, title row included: a closed section is `header` high, an open one
/// `header` and its body. A body keeps its preferred height within its limits; sections without one share what the
/// others leave. When the preferred heights do not fit, the last open sections give way first, down to their minimum.
pub fn section_heights(sections: &[StackSection], total: f32, header: f32) -> Vec<f32> {
    let room = (total - header * sections.len() as f32).max(0.0);
    let mut bodies: Vec<f32> = sections
        .iter()
        .map(|section| if section.open { section.height.map_or(0.0, |height| height.clamp(section.min, section.max.max(section.min))) } else { 0.0 })
        .collect();
    let free: Vec<usize> = (0..sections.len()).filter(|&at| sections[at].open && sections[at].height.is_none()).collect();
    let fixed: f32 = bodies.iter().sum();
    if !free.is_empty() {
        let mut leftover = (room - fixed).max(0.0);
        let mut pending = free;
        // A section held by its limit leaves its share to the others: settle the held ones, then share what remains.
        while !pending.is_empty() {
            let share = leftover / pending.len() as f32;
            let held: Vec<usize> =
                pending.iter().copied().filter(|&at| share < sections[at].min || share > sections[at].max.max(sections[at].min)).collect();
            if held.is_empty() {
                pending.iter().for_each(|&at| bodies[at] = share);
                break;
            }
            for at in held {
                let section = sections[at];
                bodies[at] = share.clamp(section.min, section.max.max(section.min));
                leftover = (leftover - bodies[at]).max(0.0);
                pending.retain(|&other| other != at);
            }
        }
    }
    let mut excess = bodies.iter().sum::<f32>() - room;
    for at in (0..sections.len()).rev() {
        if excess <= 0.0 {
            break;
        }
        let taken = excess.min((bodies[at] - sections[at].min).max(0.0));
        bodies[at] -= taken;
        excess -= taken;
    }
    sections.iter().zip(bodies).map(|(section, body)| header + if section.open { body } else { 0.0 }).collect()
}
