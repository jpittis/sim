pub struct TokenBucket {
    current: usize,
    max: usize,
}

impl TokenBucket {
    pub fn new(size: usize) -> Self {
        Self {
            current: size,
            max: size,
        }
    }

    pub fn acquire(&mut self, amount: usize) -> bool {
        if self.current >= amount {
            self.current -= amount;
            return true;
        }
        false
    }

    pub fn release(&mut self, amount: usize) {
        if self.current + amount >= self.max {
            self.current = self.max;
        } else {
            self.current += amount;
        }
    }
}
