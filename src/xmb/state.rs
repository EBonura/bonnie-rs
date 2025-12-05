//! XMB State Management
//!
//! Manages the current state of the XMB menu including selection and animations

use super::menu::{XMBAction, XMBCategory, create_default_menu};

/// XMB menu state with selection tracking and animation values
pub struct XMBState {
    /// All menu categories
    pub categories: Vec<XMBCategory>,
    /// Currently selected category index
    pub selected_category: usize,
    /// Currently selected item index within category
    pub selected_item: usize,
    /// Horizontal scroll animation value (0.0 = leftmost category)
    pub category_scroll: f32,
    /// Vertical scroll animation value (0.0 = topmost item)
    pub item_scroll: f32,
    /// Time accumulator for animations (in seconds)
    pub time: f32,
    /// Selection pulse animation (0.0 to 1.0)
    pub pulse: f32,
}

impl XMBState {
    /// Create a new XMB state with default menu
    pub fn new() -> Self {
        Self {
            categories: create_default_menu(),
            selected_category: 0,
            selected_item: 0,
            category_scroll: 0.0,
            item_scroll: 0.0,
            time: 0.0,
            pulse: 0.0,
        }
    }

    /// Create XMB state with custom categories
    pub fn with_categories(categories: Vec<XMBCategory>) -> Self {
        Self {
            categories,
            selected_category: 0,
            selected_item: 0,
            category_scroll: 0.0,
            item_scroll: 0.0,
            time: 0.0,
            pulse: 0.0,
        }
    }

    /// Update animations (call once per frame with delta time)
    pub fn update(&mut self, dt: f32) {
        self.time += dt;

        // Update pulse animation (sine wave 0.0 to 1.0)
        self.pulse = (self.time * 3.0).sin() * 0.5 + 0.5;

        // Smooth scroll to target positions (cubic ease-out)
        let target_category = self.selected_category as f32;
        let target_item = self.selected_item as f32;

        self.category_scroll = Self::ease_towards(self.category_scroll, target_category, dt * 8.0);
        self.item_scroll = Self::ease_towards(self.item_scroll, target_item, dt * 10.0);
    }

    /// Smooth easing function
    fn ease_towards(current: f32, target: f32, speed: f32) -> f32 {
        current + (target - current) * speed.min(1.0)
    }

    /// Move selection left (previous category)
    pub fn move_left(&mut self) {
        if self.selected_category > 0 {
            self.selected_category -= 1;
            self.selected_item = 0; // Reset to first item in new category
        }
    }

    /// Move selection right (next category)
    pub fn move_right(&mut self) {
        if self.selected_category < self.categories.len().saturating_sub(1) {
            self.selected_category += 1;
            self.selected_item = 0; // Reset to first item in new category
        }
    }

    /// Move selection up (previous item)
    pub fn move_up(&mut self) {
        if self.selected_item > 0 {
            self.selected_item -= 1;
        }
    }

    /// Move selection down (next item)
    pub fn move_down(&mut self) {
        let current_category = &self.categories[self.selected_category];
        if self.selected_item < current_category.items.len().saturating_sub(1) {
            self.selected_item += 1;
        }
    }

    /// Get the action of the currently selected item
    pub fn get_selected_action(&self) -> XMBAction {
        if let Some(category) = self.categories.get(self.selected_category) {
            if let Some(item) = category.items.get(self.selected_item) {
                return item.action.clone();
            }
        }
        XMBAction::None
    }

    /// Get the currently selected item's description
    pub fn get_selected_description(&self) -> Option<&str> {
        self.categories
            .get(self.selected_category)
            .and_then(|cat| cat.items.get(self.selected_item))
            .and_then(|item| item.description.as_deref())
    }

    /// Get the currently selected category
    pub fn get_selected_category(&self) -> Option<&XMBCategory> {
        self.categories.get(self.selected_category)
    }

    /// Get the currently selected item
    pub fn get_selected_item(&self) -> Option<&super::menu::XMBItem> {
        self.categories
            .get(self.selected_category)
            .and_then(|cat| cat.items.get(self.selected_item))
    }
}

impl Default for XMBState {
    fn default() -> Self {
        Self::new()
    }
}
