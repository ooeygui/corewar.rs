#[cfg(not(feature = "std"))]
use alloc::collections::VecDeque;
#[cfg(feature = "std")]
use std::collections::VecDeque;

/// Circular process queue for a single warrior.
#[derive(Debug, Clone, Default)]
pub struct ProcessQueue {
    queue: VecDeque<usize>,
}

impl ProcessQueue {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    pub fn push(&mut self, pc: usize) {
        self.queue.push_back(pc);
    }

    pub fn pop(&mut self) -> Option<usize> {
        self.queue.pop_front()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn is_full(&self, max_processes: usize) -> bool {
        self.len() >= max_processes
    }
}

#[cfg(test)]
mod tests {
    use super::ProcessQueue;

    #[test]
    fn queue_is_fifo() {
        let mut queue = ProcessQueue::new();
        queue.push(10);
        queue.push(20);

        assert_eq!(queue.pop(), Some(10));
        assert_eq!(queue.pop(), Some(20));
        assert_eq!(queue.pop(), None);
    }

    #[test]
    fn queue_reports_capacity_state() {
        let mut queue = ProcessQueue::new();
        assert!(queue.is_empty());
        assert!(!queue.is_full(2));

        queue.push(1);
        queue.push(2);

        assert_eq!(queue.len(), 2);
        assert!(queue.is_full(2));
    }
}
