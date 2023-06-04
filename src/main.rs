use rand::distributions::{Distribution, Uniform};
use std::cmp::Ordering;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::time::{Duration, Instant};

#[derive(PartialEq, Eq, Debug, Clone)]
enum Event {
    ProduceRequest(ProduceRequest),
    Request(Request),
}

#[derive(PartialEq, Eq, Debug, Clone)]
struct ProduceRequest {
    ready_at: Instant,
    interval: Duration,
    latency: Duration,
    jitter: Duration,
}

impl ProduceRequest {
    fn trigger(&self) -> Vec<Event> {
        // Generate the next request producer at a fixed interval.
        let mut next = self.clone();
        next.ready_at += self.interval;

        // Generate a request with a random latency.
        let between = Uniform::from(self.latency - self.jitter..self.latency + self.jitter);
        let mut rng = rand::thread_rng();
        let latency = between.sample(&mut rng);
        let request = Request {
            ready_at: self.ready_at + latency,
        };

        vec![Event::ProduceRequest(next), Event::Request(request)]
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
struct Request {
    ready_at: Instant,
}

impl Request {
    fn trigger(&self) -> Vec<Event> {
        println!("Request! ({:?})", self.ready_at);
        vec![]
    }
}

impl Event {
    fn ready_at(&self) -> Instant {
        match self {
            Event::ProduceRequest(inner) => inner.ready_at,
            Event::Request(inner) => inner.ready_at,
        }
    }

    fn trigger(&self) -> Vec<Event> {
        use Event::*;
        match self {
            ProduceRequest(inner) => inner.trigger(),
            Request(inner) => inner.trigger(),
        }
    }
}

impl Ord for Event {
    fn cmp(&self, other: &Self) -> Ordering {
        Reverse(self.ready_at()).cmp(&Reverse(other.ready_at()))
    }
}

impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// trait Event {
//     fn ready_at(&self) -> Instant;
// }

// struct Event(Box<dyn Event>)
// impl Eq for Box<dyn Event> {}

fn main() {
    let mut heap: BinaryHeap<Event> = BinaryHeap::new();
    heap.push(Event::ProduceRequest(ProduceRequest {
        ready_at: Instant::now(),
        interval: Duration::from_secs(1),
        latency: Duration::from_millis(100),
        jitter: Duration::from_millis(50),
    }));
    while let Some(event) = heap.pop() {
        let new_events = event.trigger();
        for new_event in new_events {
            heap.push(new_event);
        }
    }
}
