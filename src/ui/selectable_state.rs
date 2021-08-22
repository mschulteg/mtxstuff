pub(crate) trait SelectableState {
    fn select(&mut self, index: Option<usize>);
    fn selected(&self) -> Option<usize>;
    fn length(&self) -> usize;

    fn selectable(&self) -> bool {
        true
    }

    fn try_enter(&mut self) -> bool {
        if self.length() != 0 {
            self.select(Some(0));
            true
        } else {
            false
        }
    }

    fn leave(&mut self) {
        self.select(None);
    }

    fn navigate_down(&mut self) -> Option<bool> {
        if !self.selectable() {
            return None;
        }
        if let Some(selected) = self.selected() {
            if selected >= self.length() - 1 {
                self.select(Some(self.length() - 1));
                Some(false)
            } else {
                self.select(Some(selected + 1));
                Some(true)
            }
        } else {
            None
        }
    }

    fn navigate_up(&mut self) -> Option<bool> {
        if !self.selectable() {
            return None;
        }
        if let Some(selected) = self.selected() {
            if selected > 0 {
                self.select(Some(selected - 1));
                Some(true)
            } else {
                self.select(Some(0));
                Some(false)
            }
        } else {
            None
        }
    }
}
