use std::cmp::Ordering;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::time::Instant;

pub trait Handler<S>: HandlerClone<S> {
    fn call(&self, now: Instant, state: &mut S) -> Vec<Event<S>>;
}

pub trait HandlerClone<S> {
    fn clone_box(&self) -> Box<dyn Handler<S>>;
}

impl<S, T> HandlerClone<S> for T
where
    T: 'static + Handler<S> + Clone,
{
    fn clone_box(&self) -> Box<dyn Handler<S>> {
        Box::new(self.clone())
    }
}

impl<S> Clone for Box<dyn Handler<S>> {
    fn clone(&self) -> Box<dyn Handler<S>> {
        self.clone_box()
    }
}

#[derive(Clone)]
pub struct Event<S> {
    pub ready_at: Instant,
    pub handler: Box<dyn Handler<S>>,
}

impl<S> Eq for Event<S> {}

impl<S> PartialEq for Event<S> {
    fn eq(&self, other: &Self) -> bool {
        self.ready_at == other.ready_at
    }
}

impl<S> Ord for Event<S> {
    fn cmp(&self, other: &Self) -> Ordering {
        Reverse(self.ready_at).cmp(&Reverse(other.ready_at))
    }
}

impl<S> PartialOrd for Event<S> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn execute<S>(state: &mut S, init_events: Vec<Event<S>>, finish_at: Instant) {
    let mut heap = BinaryHeap::new();
    for event in init_events {
        heap.push(event);
    }
    while let Some(event) = heap.pop() {
        if event.ready_at > finish_at {
            return;
        }
        let new_events = event.handler.call(event.ready_at, state);
        for new_event in new_events {
            heap.push(new_event);
        }
    }
}
