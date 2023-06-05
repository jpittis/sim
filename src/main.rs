mod chart;
mod sim;
mod token_bucket;

use crate::sim::{execute, Event, Handler};
use crate::token_bucket::TokenBucket;
use rand::distributions::{Distribution, Uniform};
use rand::rngs::ThreadRng;
use rand::Rng;
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(PartialEq, Eq, Debug, Clone)]
struct Config {
    backoff: Duration,
    latency: Duration,
    jitter: Duration,
    bucket_size: usize,
    acquire_retry: usize,
    refill_success: usize,
    disable_token_bucket: bool,
}

struct Stats {
    counters: HashMap<String, usize>,
}

impl Stats {
    fn new() -> Self {
        Self {
            counters: HashMap::new(),
        }
    }

    fn incr(&mut self, name: &str) {
        self.counters
            .entry(name.to_string())
            .and_modify(|count| *count += 1)
            .or_insert(1);
    }

    fn get(&self, name: &str) -> usize {
        *self.counters.get(name).unwrap_or(&0)
    }
}

struct State {
    rng: ThreadRng,
    config: Config,
    stats: Stats,
    backends: Vec<bool>,
    next_round_robin: usize,
    token_bucket: TokenBucket,
}

impl State {
    fn new(config: Config, backends: Vec<bool>, stats: Stats) -> Self {
        let token_bucket = TokenBucket::new(config.bucket_size);
        Self {
            rng: rand::thread_rng(),
            config,
            stats,
            backends,
            next_round_robin: 0,
            token_bucket,
        }
    }

    fn request_latency(&mut self) -> Duration {
        let between = Uniform::from(
            self.config.latency - self.config.jitter..self.config.latency + self.config.jitter,
        );
        between.sample(&mut self.rng)
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
struct ProduceRequest {
    interval: Duration,
}

impl Handler<State> for ProduceRequest {
    fn call(&self, now: Instant, state: &mut State) -> Vec<Event<State>> {
        vec![self.request(now, state)]
    }
}

impl ProduceRequest {
    fn next_interval(&self, now: Instant) -> Event<State> {
        let clone = self.clone();
        Event {
            ready_at: now + self.interval,
            handler: Box::new(clone),
        }
    }

    fn request(&self, now: Instant, state: &mut State) -> Event<State> {
        let target = state.next_round_robin % state.backends.len();
        state.next_round_robin += 1;

        let offset = state.rng.gen_range(0..1) + 1;
        let retry_target = (target + offset) % state.backends.len();

        let latency = state.request_latency();

        Event {
            ready_at: now + latency,
            handler: Box::new(Request {
                target,
                retry_target,
                state: RequestState::Sending,
                worker: self.clone(),
            }),
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
struct Request {
    target: usize,
    retry_target: usize,
    state: RequestState,
    worker: ProduceRequest,
}

#[derive(PartialEq, Eq, Debug, Clone)]
enum RequestState {
    Sending,
    Backoff,
    Retrying,
}

impl Handler<State> for Request {
    fn call(&self, now: Instant, state: &mut State) -> Vec<Event<State>> {
        use RequestState::*;
        match self.state {
            Sending => {
                if state.backends[self.target] {
                    state.stats.incr("op_success");
                    state.stats.incr("client_success");
                    state.token_bucket.release(state.config.refill_success);
                    return vec![self.worker.next_interval(now)];
                } else {
                    state.stats.incr("op_failure");
                }

                if state.token_bucket.acquire(state.config.acquire_retry)
                    || state.config.disable_token_bucket
                {
                    let mut cloned = self.clone();
                    cloned.state = RequestState::Backoff;
                    return vec![Event {
                        ready_at: now + state.config.backoff,
                        handler: Box::new(cloned),
                    }];
                }

                state.stats.incr("client_failure");
                vec![self.worker.next_interval(now)]
            }
            Backoff => {
                let mut cloned = self.clone();
                cloned.state = RequestState::Retrying;
                let latency = state.request_latency();
                vec![Event {
                    ready_at: now + latency,
                    handler: Box::new(cloned),
                }]
            }
            Retrying => {
                if state.backends[self.retry_target] {
                    state.stats.incr("op_success");
                    state.stats.incr("client_success");
                    return vec![self.worker.next_interval(now)];
                }
                state.stats.incr("op_failure");
                state.stats.incr("client_failure");
                vec![self.worker.next_interval(now)]
            }
        }
    }
}

fn main() {
    let config = Config {
        backoff: Duration::from_millis(100),
        latency: Duration::from_millis(100),
        jitter: Duration::from_millis(50),
        bucket_size: 2,
        acquire_retry: 2,
        refill_success: 1,
        disable_token_bucket: false,
    };

    let mut config_disabled = config.clone();
    config_disabled.disable_token_bucket = true;

    let (amps_with, ratios_with) = generate(config);
    let (amps_without, ratios_without) = generate(config_disabled);

    crate::chart::chart(
        amps_with,
        amps_without,
        "Amplification",
        "Retry Amplification with and without Token Bucket",
    )
    .unwrap();

    crate::chart::chart(
        ratios_with,
        ratios_without,
        "Success Ratio",
        "Success Ratio with and without Token Bucket",
    )
    .unwrap();
}

fn run(config: Config, backends: Vec<bool>) -> Stats {
    let stats = Stats::new();
    let mut state = State::new(config, backends, stats);
    let start = Instant::now();
    let finish_at = start + Duration::from_secs(200);
    let worker = Event {
        ready_at: start,
        handler: Box::new(ProduceRequest {
            interval: Duration::from_secs(1),
        }),
    };
    execute(&mut state, vec![worker], finish_at);
    state.stats
}

fn generate(config: Config) -> (Vec<f64>, Vec<f64>) {
    let scenarios = vec![
        vec![true, true, true],
        vec![false, true, true],
        vec![false, false, true],
        vec![false, false, false],
    ];

    let mut amps = Vec::new();
    let mut ratios = Vec::new();
    for scenario in scenarios {
        let stats = run(config.clone(), scenario.clone());
        let op_success = stats.get("op_success");
        let op_failure = stats.get("op_failure");
        let client_success = stats.get("client_success");
        let client_failure = stats.get("client_failure");
        let op_total = op_success + op_failure;
        let client_total = client_success + client_failure;
        let amplification = (op_total as f64) / (client_total as f64);
        let success_ratio = (client_success as f64) / (client_total as f64);
        amps.push(amplification);
        ratios.push(success_ratio);
    }

    (amps, ratios)
}

fn print_stats(stats: Stats) {
    println!("--- stats ---");
    for (name, count) in stats.counters.iter() {
        println!("{}:\t{}", name, count)
    }
    println!("--- aggregate ---");
    let op_success = stats.get("op_success");
    let op_failure = stats.get("op_failure");
    let client_success = stats.get("client_success");
    let client_failure = stats.get("client_failure");
    let op_total = op_success + op_failure;
    let client_total = client_success + client_failure;
    let success_ratio = (client_success as f64) / (client_total as f64);
    let amplification = (op_total as f64) / (client_total as f64);
    println!("op_total:\t{}", op_total);
    println!("client_total:\t{}", client_total);
    println!("success_ratio:\t{:.2}", success_ratio);
    println!("amplification:\t{:.2}", amplification);
}
